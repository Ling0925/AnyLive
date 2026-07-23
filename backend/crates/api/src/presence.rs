//! Room presence (online count) + like counters (WBS E4.4).
//!
//! In-process only for dogfood: heartbeats expire, likes accumulate until process restart.
//! Not a warehouse; clients may poll `GET /rooms/{id}/stats`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anylive_domain::{RoomId, UserId};
use tokio::sync::RwLock;

/// Drop presence if no heartbeat within this window.
pub const PRESENCE_TTL: Duration = Duration::from_secs(45);

#[derive(Clone, Default)]
pub struct PresenceStore {
    /// room → (user → last_seen)
    online: Arc<RwLock<HashMap<RoomId, HashMap<UserId, Instant>>>>,
    /// room → total like count (monotonic for process lifetime)
    likes: Arc<RwLock<HashMap<RoomId, u64>>>,
    /// room → (user → last like instant) for light per-user rate limiting
    like_cooldown: Arc<RwLock<HashMap<RoomId, HashMap<UserId, Instant>>>>,
}

impl PresenceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a heartbeat for `user` in `room`. Returns current online count after GC.
    pub async fn heartbeat(&self, room: RoomId, user: UserId) -> u64 {
        let mut guard = self.online.write().await;
        let entry = guard.entry(room).or_default();
        entry.insert(user, Instant::now());
        Self::gc_room(entry);
        entry.len() as u64
    }

    /// Online viewers with fresh heartbeats (GC applied).
    pub async fn online_count(&self, room: RoomId) -> u64 {
        let mut guard = self.online.write().await;
        let entry = guard.entry(room).or_default();
        Self::gc_room(entry);
        entry.len() as u64
    }

    /// Add one like if the user is outside the short cooldown. Returns (accepted, total).
    pub async fn like(&self, room: RoomId, user: UserId) -> (bool, u64) {
        const COOLDOWN: Duration = Duration::from_millis(400);
        {
            let mut cool = self.like_cooldown.write().await;
            let map = cool.entry(room).or_default();
            if let Some(last) = map.get(&user) {
                if last.elapsed() < COOLDOWN {
                    let total = *self.likes.read().await.get(&room).unwrap_or(&0);
                    return (false, total);
                }
            }
            map.insert(user, Instant::now());
        }
        let mut likes = self.likes.write().await;
        let e = likes.entry(room).or_insert(0);
        *e = e.saturating_add(1);
        (true, *e)
    }

    pub async fn like_count(&self, room: RoomId) -> u64 {
        *self.likes.read().await.get(&room).unwrap_or(&0)
    }

    fn gc_room(entry: &mut HashMap<UserId, Instant>) {
        entry.retain(|_, t| t.elapsed() < PRESENCE_TTL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat_counts_unique_users() {
        let s = PresenceStore::new();
        let room = RoomId::new();
        let u1 = UserId::new();
        let u2 = UserId::new();
        assert_eq!(s.heartbeat(room, u1).await, 1);
        assert_eq!(s.heartbeat(room, u1).await, 1);
        assert_eq!(s.heartbeat(room, u2).await, 2);
        assert_eq!(s.online_count(room).await, 2);
    }

    #[tokio::test]
    async fn likes_increment_with_cooldown() {
        let s = PresenceStore::new();
        let room = RoomId::new();
        let u = UserId::new();
        let (ok, n) = s.like(room, u).await;
        assert!(ok);
        assert_eq!(n, 1);
        let (ok2, n2) = s.like(room, u).await;
        assert!(!ok2);
        assert_eq!(n2, 1);
        tokio::time::sleep(Duration::from_millis(450)).await;
        let (ok3, n3) = s.like(room, u).await;
        assert!(ok3);
        assert_eq!(n3, 2);
    }
}
