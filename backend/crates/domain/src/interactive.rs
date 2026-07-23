//! Co-host invite + PK session domain (P3). Pure state machines, no IO.

use crate::{RoomId, Timestamp, UserId};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Co-host / interactive seat ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveStatus {
    Invited,
    Active,
    Declined,
    Ended,
}

impl InteractiveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Active => "active",
            Self::Declined => "declined",
            Self::Ended => "ended",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "invited" => Some(Self::Invited),
            "active" => Some(Self::Active),
            "declined" => Some(Self::Declined),
            "ended" => Some(Self::Ended),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Declined | Self::Ended)
    }

    pub fn is_open(self) -> bool {
        matches!(self, Self::Invited | Self::Active)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractiveSession {
    pub id: Uuid,
    pub room_id: RoomId,
    pub host_id: UserId,
    pub invitee_id: UserId,
    pub status: InteractiveStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub ended_at: Option<Timestamp>,
}

impl InteractiveSession {
    pub fn invite(room_id: RoomId, host_id: UserId, invitee_id: UserId) -> Result<Self, InteractiveError> {
        if host_id == invitee_id {
            return Err(InteractiveError::SelfInvite);
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            room_id,
            host_id,
            invitee_id,
            status: InteractiveStatus::Invited,
            created_at: now,
            updated_at: now,
            ended_at: None,
        })
    }

    pub fn accept(&mut self, actor: UserId) -> Result<(), InteractiveError> {
        if actor != self.invitee_id {
            return Err(InteractiveError::NotInvitee);
        }
        if self.status != InteractiveStatus::Invited {
            return Err(InteractiveError::InvalidTransition {
                from: self.status,
                to: InteractiveStatus::Active,
            });
        }
        self.status = InteractiveStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn decline(&mut self, actor: UserId) -> Result<(), InteractiveError> {
        if actor != self.invitee_id {
            return Err(InteractiveError::NotInvitee);
        }
        if self.status != InteractiveStatus::Invited {
            return Err(InteractiveError::InvalidTransition {
                from: self.status,
                to: InteractiveStatus::Declined,
            });
        }
        let now = Utc::now();
        self.status = InteractiveStatus::Declined;
        self.updated_at = now;
        self.ended_at = Some(now);
        Ok(())
    }

    /// Host or invitee may end an invited/active session.
    pub fn end(&mut self, actor: UserId) -> Result<(), InteractiveError> {
        if actor != self.host_id && actor != self.invitee_id {
            return Err(InteractiveError::NotParticipant);
        }
        if self.status.is_terminal() {
            return Err(InteractiveError::InvalidTransition {
                from: self.status,
                to: InteractiveStatus::Ended,
            });
        }
        let now = Utc::now();
        self.status = InteractiveStatus::Ended;
        self.updated_at = now;
        self.ended_at = Some(now);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InteractiveError {
    #[error("cannot invite self")]
    SelfInvite,
    #[error("only invitee may respond")]
    NotInvitee,
    #[error("only host or invitee may leave")]
    NotParticipant,
    #[error("invalid transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: InteractiveStatus,
        to: InteractiveStatus,
    },
    #[error("room already has an open co-host seat")]
    SeatOccupied,
    #[error("pending invite already exists for this user")]
    DuplicateInvite,
}

// ── PK battle ───────────────────────────────────────────────────────────────

pub const DEFAULT_PK_DURATION_SECS: i64 = 180;
pub const MIN_PK_DURATION_SECS: i64 = 30;
pub const MAX_PK_DURATION_SECS: i64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PkStatus {
    Active,
    Ended,
}

impl PkStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Ended => "ended",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "ended" => Some(Self::Ended),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PkSession {
    pub id: Uuid,
    pub room_a_id: RoomId,
    pub room_b_id: RoomId,
    pub host_a_id: UserId,
    pub host_b_id: UserId,
    pub status: PkStatus,
    pub score_a: i64,
    pub score_b: i64,
    pub winner_room_id: Option<RoomId>,
    pub started_at: Timestamp,
    pub ends_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub updated_at: Timestamp,
}

impl PkSession {
    pub fn start(
        room_a_id: RoomId,
        host_a_id: UserId,
        room_b_id: RoomId,
        host_b_id: UserId,
        duration_secs: i64,
    ) -> Result<Self, PkError> {
        if room_a_id == room_b_id {
            return Err(PkError::SameRoom);
        }
        if host_a_id == host_b_id {
            return Err(PkError::SameHost);
        }
        let duration_secs = duration_secs.clamp(MIN_PK_DURATION_SECS, MAX_PK_DURATION_SECS);
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            room_a_id,
            room_b_id,
            host_a_id,
            host_b_id,
            status: PkStatus::Active,
            score_a: 0,
            score_b: 0,
            winner_room_id: None,
            started_at: now,
            ends_at: now + Duration::seconds(duration_secs),
            ended_at: None,
            updated_at: now,
        })
    }

    pub fn involves_room(&self, room_id: RoomId) -> bool {
        self.room_a_id == room_id || self.room_b_id == room_id
    }

    pub fn is_host(&self, user: UserId) -> bool {
        self.host_a_id == user || self.host_b_id == user
    }

    /// Add gift coins to the room's score while PK is active.
    pub fn add_score(&mut self, room_id: RoomId, coins: i64) -> Result<(), PkError> {
        if self.status != PkStatus::Active {
            return Err(PkError::NotActive);
        }
        if coins < 0 {
            return Err(PkError::NegativeScore);
        }
        if room_id == self.room_a_id {
            self.score_a = self.score_a.saturating_add(coins);
        } else if room_id == self.room_b_id {
            self.score_b = self.score_b.saturating_add(coins);
        } else {
            return Err(PkError::RoomNotInPk);
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    /// End PK (host early-end or timer). Sets winner by higher score; tie → None.
    pub fn end(&mut self) -> Result<(), PkError> {
        if self.status != PkStatus::Active {
            return Err(PkError::NotActive);
        }
        let now = Utc::now();
        self.status = PkStatus::Ended;
        self.ended_at = Some(now);
        self.updated_at = now;
        self.winner_room_id = if self.score_a > self.score_b {
            Some(self.room_a_id)
        } else if self.score_b > self.score_a {
            Some(self.room_b_id)
        } else {
            None
        };
        Ok(())
    }

    /// If timer elapsed and still active, end automatically.
    pub fn maybe_expire(&mut self) -> bool {
        if self.status == PkStatus::Active && Utc::now() >= self.ends_at {
            let _ = self.end();
            return true;
        }
        false
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PkError {
    #[error("cannot PK the same room")]
    SameRoom,
    #[error("cannot PK same host")]
    SameHost,
    #[error("PK is not active")]
    NotActive,
    #[error("room is not part of this PK")]
    RoomNotInPk,
    #[error("score delta must be non-negative")]
    NegativeScore,
    #[error("room already in an active PK")]
    AlreadyInPk,
    #[error("opponent room is not live")]
    OpponentNotLive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_accept_leave() {
        let host = UserId::new();
        let guest = UserId::new();
        let mut s = InteractiveSession::invite(RoomId::new(), host, guest).unwrap();
        assert_eq!(s.status, InteractiveStatus::Invited);
        s.accept(guest).unwrap();
        assert_eq!(s.status, InteractiveStatus::Active);
        s.end(host).unwrap();
        assert_eq!(s.status, InteractiveStatus::Ended);
        assert!(s.ended_at.is_some());
    }

    #[test]
    fn invitee_only_accepts() {
        let host = UserId::new();
        let guest = UserId::new();
        let mut s = InteractiveSession::invite(RoomId::new(), host, guest).unwrap();
        assert_eq!(s.accept(host).unwrap_err(), InteractiveError::NotInvitee);
        s.decline(guest).unwrap();
        assert_eq!(s.status, InteractiveStatus::Declined);
    }

    #[test]
    fn self_invite_rejected() {
        let u = UserId::new();
        assert_eq!(
            InteractiveSession::invite(RoomId::new(), u, u).unwrap_err(),
            InteractiveError::SelfInvite
        );
    }

    #[test]
    fn pk_scores_and_winner() {
        let a = RoomId::new();
        let b = RoomId::new();
        let mut pk = PkSession::start(a, UserId::new(), b, UserId::new(), 120).unwrap();
        pk.add_score(a, 100).unwrap();
        pk.add_score(b, 50).unwrap();
        pk.end().unwrap();
        assert_eq!(pk.winner_room_id, Some(a));
        assert_eq!(pk.status, PkStatus::Ended);
    }

    #[test]
    fn pk_tie_no_winner() {
        let a = RoomId::new();
        let b = RoomId::new();
        let mut pk = PkSession::start(a, UserId::new(), b, UserId::new(), 60).unwrap();
        pk.add_score(a, 10).unwrap();
        pk.add_score(b, 10).unwrap();
        pk.end().unwrap();
        assert_eq!(pk.winner_room_id, None);
    }

    #[test]
    fn pk_same_room_rejected() {
        let r = RoomId::new();
        assert_eq!(
            PkSession::start(r, UserId::new(), r, UserId::new(), 60).unwrap_err(),
            PkError::SameRoom
        );
    }
}
