use crate::{RoomId, Timestamp, UserId};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    Idle,
    Live,
    Closed,
}

impl RoomStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Live => "live",
            Self::Closed => "closed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Self::Idle),
            "live" => Some(Self::Live),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }

    /// Whether the room can transition to `next`.
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Idle, Self::Live)
                | (Self::Live, Self::Idle)
                | (Self::Live, Self::Closed)
                | (Self::Idle, Self::Closed)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Room {
    pub id: RoomId,
    pub owner_id: UserId,
    pub title: String,
    pub status: RoomStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Room {
    pub fn new(owner_id: UserId, title: impl Into<String>) -> Result<Self, RoomError> {
        let title = title.into();
        let title = title.trim().to_string();
        if title.is_empty() || title.len() > 80 {
            return Err(RoomError::InvalidTitle);
        }
        let now = Utc::now();
        Ok(Self {
            id: RoomId::new(),
            owner_id,
            title,
            status: RoomStatus::Idle,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn transition(&mut self, next: RoomStatus) -> Result<(), RoomError> {
        if !self.status.can_transition_to(next) {
            return Err(RoomError::InvalidTransition {
                from: self.status,
                to: next,
            });
        }
        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RoomError {
    #[error("invalid title")]
    InvalidTitle,
    #[error("invalid transition from {from:?} to {to:?}")]
    InvalidTransition { from: RoomStatus, to: RoomStatus },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UserId;

    #[test]
    fn create_room_trims_title() {
        let room = Room::new(UserId::new(), "  hello  ").unwrap();
        assert_eq!(room.title, "hello");
        assert_eq!(room.status, RoomStatus::Idle);
    }

    #[test]
    fn reject_empty_title() {
        assert_eq!(
            Room::new(UserId::new(), "   ").unwrap_err(),
            RoomError::InvalidTitle
        );
    }

    #[test]
    fn reject_too_long_title() {
        let t = "x".repeat(81);
        assert!(Room::new(UserId::new(), t).is_err());
    }

    #[test]
    fn transitions_idle_live_closed() {
        let mut room = Room::new(UserId::new(), "show").unwrap();
        room.transition(RoomStatus::Live).unwrap();
        assert_eq!(room.status, RoomStatus::Live);
        room.transition(RoomStatus::Closed).unwrap();
        assert_eq!(room.status, RoomStatus::Closed);
        assert!(room.transition(RoomStatus::Live).is_err());
    }

    #[test]
    fn status_parse_roundtrip() {
        for s in [RoomStatus::Idle, RoomStatus::Live, RoomStatus::Closed] {
            assert_eq!(RoomStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(RoomStatus::parse("nope"), None);
    }
}
