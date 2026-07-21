//! Social graph: follow / unfollow / following feed helpers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::UserId;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct MemorySocial {
    /// follower -> set of followees
    inner: Arc<Mutex<HashMap<Uuid, HashSet<Uuid>>>>,
}

impl MemorySocial {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn follow(&self, follower: UserId, followee: UserId) -> Result<(), AppError> {
        if follower.0 == followee.0 {
            return Err(AppError::validation("cannot follow yourself"));
        }
        let mut g = self.inner.lock().await;
        g.entry(follower.0).or_default().insert(followee.0);
        Ok(())
    }

    pub async fn unfollow(&self, follower: UserId, followee: UserId) -> Result<(), AppError> {
        let mut g = self.inner.lock().await;
        if let Some(set) = g.get_mut(&follower.0) {
            set.remove(&followee.0);
        }
        Ok(())
    }

    pub async fn is_following(&self, follower: UserId, followee: UserId) -> bool {
        let g = self.inner.lock().await;
        g.get(&follower.0)
            .map(|s| s.contains(&followee.0))
            .unwrap_or(false)
    }

    pub async fn following_ids(&self, follower: UserId) -> Vec<UserId> {
        let g = self.inner.lock().await;
        g.get(&follower.0)
            .map(|s| s.iter().copied().map(UserId).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn follow_unfollow() {
        let s = MemorySocial::new();
        let a = UserId::new();
        let b = UserId::new();
        s.follow(a, b).await.unwrap();
        assert!(s.is_following(a, b).await);
        assert_eq!(s.following_ids(a).await.len(), 1);
        s.unfollow(a, b).await.unwrap();
        assert!(!s.is_following(a, b).await);
    }

    #[tokio::test]
    async fn reject_self_follow() {
        let s = MemorySocial::new();
        let a = UserId::new();
        assert!(s.follow(a, a).await.is_err());
    }
}
