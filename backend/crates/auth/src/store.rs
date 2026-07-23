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
    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError>;
    /// Update display name for an existing user; returns the updated user.
    async fn update_display_name(
        &self,
        id: UserId,
        display_name: String,
    ) -> Result<User, AppError>;
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
}

#[async_trait]
impl UserStore for InMemoryUserStore {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let map = self.by_email.read().await;
        let Some(id) = map.get(email) else {
            return Ok(None);
        };
        Ok(self.by_id.read().await.get(id).cloned())
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        Ok(self.by_id.read().await.get(&id.0).cloned())
    }

    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError> {
        // Hold both write locks for the check-and-insert to avoid duplicate users
        // under concurrent first-login races.
        let mut by_email = self.by_email.write().await;
        if let Some(id) = by_email.get(email) {
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
        let user = User::new(display, Some(email.to_string())).map_err(|e| {
            AppError::validation(format!("cannot create user: {e}"))
        })?;
        by_email.insert(email.to_string(), user.id.0);
        self.by_id.write().await.insert(user.id.0, user.clone());
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
