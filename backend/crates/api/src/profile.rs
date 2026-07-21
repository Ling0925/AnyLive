//! In-memory profile extras: age confirmation + privacy acceptance.
//!
//! Kept off the `users` table so Postgres needs no migration for P1.
//! Dual-store pattern: display_name lives on UserStore; these flags live here.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_domain::UserId;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Per-user age / privacy declaration timestamps.
#[derive(Debug, Clone, Default)]
pub struct ProfileExtras {
    pub age_confirmed_at: Option<DateTime<Utc>>,
    pub privacy_accepted_at: Option<DateTime<Utc>>,
}

impl ProfileExtras {
    pub fn age_confirmed(&self) -> bool {
        self.age_confirmed_at.is_some()
    }

    pub fn privacy_accepted(&self) -> bool {
        self.privacy_accepted_at.is_some()
    }
}

/// Process-local store for age/privacy flags (memory dual of UserStore extras).
#[derive(Clone, Default)]
pub struct MemoryProfileExtras {
    inner: Arc<RwLock<HashMap<Uuid, ProfileExtras>>>,
}

impl MemoryProfileExtras {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, user_id: UserId) -> ProfileExtras {
        self.inner
            .read()
            .await
            .get(&user_id.0)
            .cloned()
            .unwrap_or_default()
    }

    /// Set age confirmation. `true` stamps now; `false` clears.
    pub async fn set_age_confirmed(&self, user_id: UserId, confirmed: bool) -> ProfileExtras {
        let mut g = self.inner.write().await;
        let entry = g.entry(user_id.0).or_default();
        entry.age_confirmed_at = if confirmed {
            Some(Utc::now())
        } else {
            None
        };
        entry.clone()
    }

    /// Set privacy acceptance. `true` stamps now; `false` clears.
    pub async fn set_privacy_accepted(&self, user_id: UserId, accepted: bool) -> ProfileExtras {
        let mut g = self.inner.write().await;
        let entry = g.entry(user_id.0).or_default();
        entry.privacy_accepted_at = if accepted {
            Some(Utc::now())
        } else {
            None
        };
        entry.clone()
    }

    /// Apply optional age/privacy patches and return the resulting extras.
    pub async fn patch(
        &self,
        user_id: UserId,
        age_confirmed: Option<bool>,
        privacy_accepted: Option<bool>,
    ) -> ProfileExtras {
        let mut g = self.inner.write().await;
        let entry = g.entry(user_id.0).or_default();
        if let Some(v) = age_confirmed {
            entry.age_confirmed_at = if v { Some(Utc::now()) } else { None };
        }
        if let Some(v) = privacy_accepted {
            entry.privacy_accepted_at = if v { Some(Utc::now()) } else { None };
        }
        entry.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn defaults_false() {
        let s = MemoryProfileExtras::new();
        let e = s.get(UserId::new()).await;
        assert!(!e.age_confirmed());
        assert!(!e.privacy_accepted());
    }

    #[tokio::test]
    async fn patch_sets_and_clears() {
        let s = MemoryProfileExtras::new();
        let id = UserId::new();
        let e = s.patch(id, Some(true), Some(true)).await;
        assert!(e.age_confirmed());
        assert!(e.privacy_accepted());
        assert!(e.age_confirmed_at.is_some());
        assert!(e.privacy_accepted_at.is_some());

        let e = s.patch(id, Some(false), None).await;
        assert!(!e.age_confirmed());
        assert!(e.privacy_accepted());
    }
}
