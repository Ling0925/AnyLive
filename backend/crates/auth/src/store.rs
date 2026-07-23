//! Storage traits + in-memory implementations (no Postgres required).

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::{User, UserId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OtpChallenge {
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub attempts: u32,
}

#[async_trait]
pub trait OtpStore: Send + Sync + Clone {
    async fn put(&self, email: &str, challenge: OtpChallenge) -> Result<(), AppError>;
    async fn get(&self, email: &str) -> Result<Option<OtpChallenge>, AppError>;
    async fn take(&self, email: &str) -> Result<Option<OtpChallenge>, AppError>;
}

#[async_trait]
pub trait UserStore: Send + Sync + Clone {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError>;
    /// Case-insensitive username lookup.
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError>;
    /// Create a provisioned user (admin open-account). Fails if email/username conflicts.
    async fn create_user(&self, user: User) -> Result<User, AppError>;
    /// Update display name for an existing user; returns the updated user.
    async fn update_display_name(
        &self,
        id: UserId,
        display_name: String,
    ) -> Result<User, AppError>;
    /// Patch mutable account fields (admin).
    async fn update_account(
        &self,
        id: UserId,
        display_name: Option<String>,
        email: Option<Option<String>>,
        username: Option<Option<String>>,
        status: Option<anylive_domain::UserStatus>,
    ) -> Result<User, AppError>;
    /// Admin list/search (username, email, display_name).
    async fn list_users(
        &self,
        q: Option<&str>,
        status: Option<anylive_domain::UserStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<User>, usize), AppError>;
}

/// Active refresh tokens keyed by jti.
#[async_trait]
pub trait RefreshStore: Send + Sync + Clone {
    async fn insert(&self, jti: Uuid, user_id: UserId, exp: i64) -> Result<(), AppError>;
    async fn revoke(&self, jti: Uuid) -> Result<bool, AppError>;
    async fn is_active(&self, jti: Uuid) -> Result<bool, AppError>;
    /// Revoke all refresh tokens for a user (logout-all style).
    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<usize, AppError>;
}

// --- In-memory ---

#[derive(Clone, Default)]
pub struct InMemoryOtpStore {
    inner: Arc<RwLock<HashMap<String, OtpChallenge>>>,
}

#[async_trait]
impl OtpStore for InMemoryOtpStore {
    async fn put(&self, email: &str, challenge: OtpChallenge) -> Result<(), AppError> {
        self.inner.write().await.insert(email.to_string(), challenge);
        Ok(())
    }

    async fn get(&self, email: &str) -> Result<Option<OtpChallenge>, AppError> {
        Ok(self.inner.read().await.get(email).cloned())
    }

    async fn take(&self, email: &str) -> Result<Option<OtpChallenge>, AppError> {
        Ok(self.inner.write().await.remove(email))
    }
}

#[derive(Clone, Default)]
pub struct InMemoryUserStore {
    by_id: Arc<RwLock<HashMap<Uuid, User>>>,
    by_email: Arc<RwLock<HashMap<String, Uuid>>>,
    by_username: Arc<RwLock<HashMap<String, Uuid>>>,
}

#[async_trait]
impl UserStore for InMemoryUserStore {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let email = email.trim().to_lowercase();
        let map = self.by_email.read().await;
        let Some(id) = map.get(&email) else {
            return Ok(None);
        };
        Ok(self.by_id.read().await.get(id).cloned())
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        Ok(self.by_id.read().await.get(&id.0).cloned())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let username = username.trim().to_ascii_lowercase();
        let map = self.by_username.read().await;
        let Some(id) = map.get(&username) else {
            return Ok(None);
        };
        Ok(self.by_id.read().await.get(id).cloned())
    }

    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError> {
        let email = email.trim().to_lowercase();
        // Hold both write locks for the check-and-insert to avoid duplicate users
        // under concurrent first-login races.
        let mut by_email = self.by_email.write().await;
        if let Some(id) = by_email.get(&email) {
            return Ok(self
                .by_id
                .read()
                .await
                .get(id)
                .cloned()
                .expect("by_email/by_id consistency"));
        }
        let local = email.split('@').next().unwrap_or("user");
        let display = if local.is_empty() {
            "user".to_string()
        } else if local.len() > 64 {
            local[..64].to_string()
        } else {
            local.to_string()
        };
        let user = User::new(display, Some(email.clone())).map_err(|e| {
            AppError::validation(format!("cannot create user: {e}"))
        })?;
        by_email.insert(email, user.id.0);
        self.by_id.write().await.insert(user.id.0, user.clone());
        Ok(user)
    }

    async fn create_user(&self, user: User) -> Result<User, AppError> {
        let mut by_id = self.by_id.write().await;
        if by_id.contains_key(&user.id.0) {
            return Err(AppError::new(
                anylive_common::ErrorCode::Conflict,
                "user id already exists",
            ));
        }
        if let Some(ref email) = user.email {
            let key = email.trim().to_lowercase();
            let mut by_email = self.by_email.write().await;
            if by_email.contains_key(&key) {
                return Err(AppError::new(
                    anylive_common::ErrorCode::Conflict,
                    "email already registered",
                ));
            }
            by_email.insert(key, user.id.0);
        }
        if let Some(ref username) = user.username {
            let key = username.trim().to_ascii_lowercase();
            let mut by_username = self.by_username.write().await;
            if by_username.contains_key(&key) {
                return Err(AppError::new(
                    anylive_common::ErrorCode::Conflict,
                    "username already taken",
                ));
            }
            by_username.insert(key, user.id.0);
        }
        by_id.insert(user.id.0, user.clone());
        Ok(user)
    }

    async fn update_display_name(
        &self,
        id: UserId,
        display_name: String,
    ) -> Result<User, AppError> {
        let name = User::validate_display_name(display_name)
            .map_err(|e| AppError::validation(format!("{e}")))?;
        let mut by_id = self.by_id.write().await;
        let user = by_id
            .get_mut(&id.0)
            .ok_or_else(|| AppError::not_found("user not found"))?;
        user.display_name = name;
        Ok(user.clone())
    }

    async fn update_account(
        &self,
        id: UserId,
        display_name: Option<String>,
        email: Option<Option<String>>,
        username: Option<Option<String>>,
        status: Option<anylive_domain::UserStatus>,
    ) -> Result<User, AppError> {
        let mut by_id = self.by_id.write().await;
        let user = by_id
            .get(&id.0)
            .cloned()
            .ok_or_else(|| AppError::not_found("user not found"))?;
        let mut next = user.clone();
        if let Some(name) = display_name {
            next.display_name = User::validate_display_name(name)
                .map_err(|e| AppError::validation(format!("{e}")))?;
        }
        if let Some(status) = status {
            next.status = status;
        }
        if let Some(email_opt) = email {
            // reindex email
            if let Some(ref old) = user.email {
                self.by_email.write().await.remove(&old.to_lowercase());
            }
            if let Some(e) = email_opt {
                let key = e.trim().to_lowercase();
                if !key.contains('@') {
                    return Err(AppError::validation("invalid email"));
                }
                let mut by_email = self.by_email.write().await;
                if let Some(other) = by_email.get(&key) {
                    if *other != id.0 {
                        return Err(AppError::new(
                            anylive_common::ErrorCode::Conflict,
                            "email already registered",
                        ));
                    }
                }
                by_email.insert(key.clone(), id.0);
                next.email = Some(key);
            } else {
                next.email = None;
            }
        }
        if let Some(username_opt) = username {
            if let Some(ref old) = user.username {
                self.by_username
                    .write()
                    .await
                    .remove(&old.to_ascii_lowercase());
            }
            if let Some(u) = username_opt {
                let key = User::validate_username(u)
                    .map_err(|e| AppError::validation(format!("{e}")))?;
                let mut by_username = self.by_username.write().await;
                if let Some(other) = by_username.get(&key) {
                    if *other != id.0 {
                        return Err(AppError::new(
                            anylive_common::ErrorCode::Conflict,
                            "username already taken",
                        ));
                    }
                }
                by_username.insert(key.clone(), id.0);
                next.username = Some(key);
            } else {
                next.username = None;
            }
        }
        by_id.insert(id.0, next.clone());
        Ok(next)
    }

    async fn list_users(
        &self,
        q: Option<&str>,
        status: Option<anylive_domain::UserStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<User>, usize), AppError> {
        let guard = self.by_id.read().await;
        let needle = q.map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty());
        let mut items: Vec<User> = guard
            .values()
            .filter(|u| status.map(|s| u.status == s).unwrap_or(true))
            .filter(|u| {
                let Some(ref n) = needle else {
                    return true;
                };
                u.display_name.to_ascii_lowercase().contains(n)
                    || u.email
                        .as_ref()
                        .map(|e| e.to_ascii_lowercase().contains(n))
                        .unwrap_or(false)
                    || u.username
                        .as_ref()
                        .map(|un| un.to_ascii_lowercase().contains(n))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let total = items.len();
        let page = items.into_iter().skip(offset).take(limit).collect();
        Ok((page, total))
    }
}

impl InMemoryUserStore {
    /// Case-insensitive substring match on display name (dogfood search).
    pub async fn search_display_name(&self, q: &str, limit: usize) -> Vec<User> {
        let needle = q.trim().to_ascii_lowercase();
        if needle.is_empty() || limit == 0 {
            return Vec::new();
        }
        let guard = self.by_id.read().await;
        let mut items: Vec<User> = guard
            .values()
            .filter(|u| u.display_name.to_ascii_lowercase().contains(&needle))
            .cloned()
            .collect();
        items.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        items.into_iter().take(limit).collect()
    }
}

#[derive(Debug, Clone)]
struct RefreshRecord {
    user_id: UserId,
    exp: i64,
}

#[derive(Clone, Default)]
pub struct InMemoryRefreshStore {
    // jti -> record; absence means revoked or never issued
    active: Arc<RwLock<HashMap<Uuid, RefreshRecord>>>,
}

#[async_trait]
impl RefreshStore for InMemoryRefreshStore {
    async fn insert(&self, jti: Uuid, user_id: UserId, exp: i64) -> Result<(), AppError> {
        self.active
            .write()
            .await
            .insert(jti, RefreshRecord { user_id, exp });
        Ok(())
    }

    async fn revoke(&self, jti: Uuid) -> Result<bool, AppError> {
        Ok(self.active.write().await.remove(&jti).is_some())
    }

    async fn is_active(&self, jti: Uuid) -> Result<bool, AppError> {
        let guard = self.active.read().await;
        Ok(match guard.get(&jti) {
            Some(rec) => rec.exp >= chrono::Utc::now().timestamp(),
            None => false,
        })
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<usize, AppError> {
        let mut guard = self.active.write().await;
        let before = guard.len();
        guard.retain(|_, rec| rec.user_id != user_id);
        Ok(before - guard.len())
    }
}

#[derive(Debug, Clone)]
pub struct RefreshSessionInfo {
    pub jti: Uuid,
    pub user_id: UserId,
    pub exp: i64,
}

impl InMemoryRefreshStore {
    /// Lookup a refresh session by jti (includes expired rows still present).
    pub async fn get(&self, jti: Uuid) -> Option<RefreshSessionInfo> {
        let guard = self.active.read().await;
        guard.get(&jti).map(|rec| RefreshSessionInfo {
            jti,
            user_id: rec.user_id,
            exp: rec.exp,
        })
    }

    /// Revoke only if the jti belongs to `user_id` (returns false when missing or other user).
    pub async fn revoke_for_user(&self, jti: Uuid, user_id: UserId) -> Result<bool, AppError> {
        let mut guard = self.active.write().await;
        match guard.get(&jti) {
            Some(rec) if rec.user_id == user_id => {
                guard.remove(&jti);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// List non-expired refresh sessions for a user (no device metadata in v0).
    pub async fn list_for_user(&self, user_id: UserId) -> Vec<RefreshSessionInfo> {
        let now = Utc::now().timestamp();
        let guard = self.active.read().await;
        let mut items: Vec<RefreshSessionInfo> = guard
            .iter()
            .filter(|(_, rec)| rec.user_id == user_id && rec.exp >= now)
            .map(|(jti, rec)| RefreshSessionInfo {
                jti: *jti,
                user_id: rec.user_id,
                exp: rec.exp,
            })
            .collect();
        items.sort_by(|a, b| b.exp.cmp(&a.exp));
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn user_upsert_is_idempotent() {
        let store = InMemoryUserStore::default();
        let a = store.upsert_by_email("a@example.com").await.unwrap();
        let b = store.upsert_by_email("a@example.com").await.unwrap();
        assert_eq!(a.id, b.id);
    }

    #[tokio::test]
    async fn update_display_name_persists() {
        let store = InMemoryUserStore::default();
        let u = store.upsert_by_email("name@example.com").await.unwrap();
        let updated = store
            .update_display_name(u.id, "New Name".into())
            .await
            .unwrap();
        assert_eq!(updated.display_name, "New Name");
        let again = store.find_by_id(u.id).await.unwrap().unwrap();
        assert_eq!(again.display_name, "New Name");
    }

    #[tokio::test]
    async fn update_display_name_rejects_empty() {
        let store = InMemoryUserStore::default();
        let u = store.upsert_by_email("bad@example.com").await.unwrap();
        let err = store
            .update_display_name(u.id, "   ".into())
            .await
            .unwrap_err();
        assert_eq!(err.code, anylive_common::ErrorCode::Validation);
    }

    #[tokio::test]
    async fn update_display_name_missing_user() {
        let store = InMemoryUserStore::default();
        let err = store
            .update_display_name(UserId::new(), "Ghost".into())
            .await
            .unwrap_err();
        assert_eq!(err.code, anylive_common::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn refresh_revoke() {
        let store = InMemoryRefreshStore::default();
        let jti = Uuid::new_v4();
        let uid = UserId::new();
        store
            .insert(jti, uid, Utc::now().timestamp() + 3600)
            .await
            .unwrap();
        assert!(store.is_active(jti).await.unwrap());
        assert!(store.revoke(jti).await.unwrap());
        assert!(!store.is_active(jti).await.unwrap());
    }

    #[tokio::test]
    async fn revoke_for_user_respects_ownership() {
        let store = InMemoryRefreshStore::default();
        let jti = Uuid::new_v4();
        let owner = UserId::new();
        let other = UserId::new();
        store
            .insert(jti, owner, Utc::now().timestamp() + 3600)
            .await
            .unwrap();
        assert!(!store.revoke_for_user(jti, other).await.unwrap());
        assert!(store.is_active(jti).await.unwrap());
        assert!(store.revoke_for_user(jti, owner).await.unwrap());
        assert!(!store.is_active(jti).await.unwrap());
    }
}
