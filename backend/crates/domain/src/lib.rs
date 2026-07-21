//! Pure domain types for AnyLive (no IO).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod room;
pub mod user;

pub use room::{Room, RoomStatus};
pub use user::User;

/// Newtype helpers for strong IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomId(pub Uuid);

impl RoomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RoomId {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared timestamp alias.
pub type Timestamp = DateTime<Utc>;
