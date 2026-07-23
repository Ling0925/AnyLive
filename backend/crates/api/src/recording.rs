//! Room recording enable/disable control plane (WBS E3.5).
//!
//! Flag only — no actual media egress. Hosts toggle; clients/admin can read it
//! on room stats / dedicated endpoint. In-process; optional Postgres dual later.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_domain::RoomId;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct RecordingStore {
    /// room → recording enabled
    flags: Arc<RwLock<HashMap<RoomId, bool>>>,
}

impl RecordingStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, room: RoomId) -> bool {
        *self.flags.read().await.get(&room).unwrap_or(&false)
    }

    pub async fn set(&self, room: RoomId, enabled: bool) -> bool {
        self.flags.write().await.insert(room, enabled);
        enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_off_then_toggle() {
        let s = RecordingStore::new();
        let r = RoomId::new();
        assert!(!s.get(r).await);
        assert!(s.set(r, true).await);
        assert!(s.get(r).await);
        assert!(!s.set(r, false).await);
        assert!(!s.get(r).await);
    }
}
