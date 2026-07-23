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

    /// Number of followers for a user (reverse edge scan).
    pub async fn follower_count(&self, user: UserId) -> u64 {
        let g = self.inner.lock().await;
        g.values()
            .filter(|set| set.contains(&user.0))
            .count() as u64
    }

    /// Number of users this account follows.
    pub async fn following_count(&self, user: UserId) -> u64 {
        let g = self.inner.lock().await;
        g.get(&user.0).map(|s| s.len() as u64).unwrap_or(0)
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
        assert_eq!(s.follower_count(b).await, 0);
    }

    #[tokio::test]
    async fn follower_count_tracks_reverse_edges() {
        let s = MemorySocial::new();
        let host = UserId::new();
        let f1 = UserId::new();
        let f2 = UserId::new();
        s.follow(f1, host).await.unwrap();
        s.follow(f2, host).await.unwrap();
        assert_eq!(s.follower_count(host).await, 2);
        assert_eq!(s.following_count(f1).await, 1);
    }

    #[tokio::test]
    async fn reject_self_follow() {
        let s = MemorySocial::new();
        let a = UserId::new();
        assert!(s.follow(a, a).await.is_err());
    }
}
