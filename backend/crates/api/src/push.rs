//! Device push token registration scaffold (WBS E8.9).
//!
//! Control plane only: register / list / unregister tokens in-process.
//! No FCM/APNs delivery — that needs vendor keys + worker.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_domain::UserId;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PushDevice {
    pub id: Uuid,
    pub user_id: UserId,
    pub platform: String,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Default)]
pub struct PushStore {
    /// token → device (unique token across users; last writer wins ownership)
    by_token: Arc<RwLock<HashMap<String, PushDevice>>>,
}

impl PushStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upsert by device token. Returns the stored device.
    pub async fn register(
        &self,
        user_id: UserId,
        platform: impl Into<String>,
        token: impl Into<String>,
    ) -> PushDevice {
        let platform = platform.into();
        let token = token.into();
        let now = Utc::now();
        let mut guard = self.by_token.write().await;
        if let Some(existing) = guard.get_mut(&token) {
            existing.user_id = user_id;
            existing.platform = platform;
            existing.updated_at = now;
            return existing.clone();
        }
        let device = PushDevice {
            id: Uuid::new_v4(),
            user_id,
            platform,
            token: token.clone(),
            created_at: now,
            updated_at: now,
        };
        guard.insert(token, device.clone());
        device
    }

    /// Remove a token if it belongs to `user_id`. Returns whether a row was removed.
    pub async fn unregister(&self, user_id: UserId, token: &str) -> bool {
        let mut guard = self.by_token.write().await;
        match guard.get(token) {
            Some(d) if d.user_id == user_id => {
                guard.remove(token);
                true
            }
            _ => false,
        }
    }

    pub async fn list_for_user(&self, user_id: UserId) -> Vec<PushDevice> {
        let guard = self.by_token.read().await;
        let mut items: Vec<PushDevice> = guard
            .values()
            .filter(|d| d.user_id == user_id)
            .cloned()
            .collect();
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        items
    }

    pub async fn count(&self) -> usize {
        self.by_token.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_upsert_and_unregister() {
        let store = PushStore::new();
        let u1 = UserId::new();
        let u2 = UserId::new();
        let a = store.register(u1, "ios", "tok-a").await;
        assert_eq!(a.platform, "ios");
        let again = store.register(u1, "ios", "tok-a").await;
        assert_eq!(a.id, again.id);
        // Token reassigned to another user
        let b = store.register(u2, "android", "tok-a").await;
        assert_eq!(b.user_id, u2);
        assert_eq!(store.list_for_user(u1).await.len(), 0);
        assert_eq!(store.list_for_user(u2).await.len(), 1);
        assert!(store.unregister(u2, "tok-a").await);
        assert!(!store.unregister(u2, "tok-a").await);
        assert_eq!(store.count().await, 0);
    }
}
