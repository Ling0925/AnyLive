//! In-memory co-host + PK session store (P3 control plane).

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{
    InteractiveError, InteractiveSession, InteractiveStatus, PkError, PkSession, RoomId, UserId,
    DEFAULT_PK_DURATION_SECS,
};
use tokio::sync::RwLock;
use uuid::Uuid;

fn map_interactive(err: InteractiveError) -> AppError {
    match err {
        InteractiveError::SelfInvite | InteractiveError::NotInvitee | InteractiveError::NotParticipant => {
            AppError::new(ErrorCode::Forbidden, err.to_string())
        }
        InteractiveError::InvalidTransition { .. } => {
            AppError::new(ErrorCode::Conflict, err.to_string())
        }
        InteractiveError::SeatOccupied | InteractiveError::DuplicateInvite => {
            AppError::new(ErrorCode::Conflict, err.to_string())
        }
    }
}

fn map_pk(err: PkError) -> AppError {
    match err {
        PkError::SameRoom | PkError::SameHost | PkError::NegativeScore | PkError::RoomNotInPk => {
            AppError::validation(err.to_string())
        }
        PkError::NotActive => AppError::not_found("no active PK"),
        PkError::AlreadyInPk | PkError::OpponentNotLive => {
            AppError::new(ErrorCode::Conflict, err.to_string())
        }
    }
}

#[derive(Clone, Default)]
pub struct InteractiveStore {
    sessions: Arc<RwLock<HashMap<Uuid, InteractiveSession>>>,
    /// room_id → active PK session id
    pk_by_room: Arc<RwLock<HashMap<RoomId, Uuid>>>,
    pks: Arc<RwLock<HashMap<Uuid, PkSession>>>,
}

impl InteractiveStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn invite(
        &self,
        room_id: RoomId,
        host_id: UserId,
        invitee_id: UserId,
    ) -> Result<InteractiveSession, AppError> {
        let mut guard = self.sessions.write().await;
        // One open seat per room (invited or active).
        if guard
            .values()
            .any(|s| s.room_id == room_id && s.status.is_open())
        {
            return Err(map_interactive(InteractiveError::SeatOccupied));
        }
        if guard.values().any(|s| {
            s.room_id == room_id
                && s.invitee_id == invitee_id
                && s.status == InteractiveStatus::Invited
        }) {
            return Err(map_interactive(InteractiveError::DuplicateInvite));
        }
        let session =
            InteractiveSession::invite(room_id, host_id, invitee_id).map_err(map_interactive)?;
        guard.insert(session.id, session.clone());
        Ok(session)
    }

    pub async fn respond(
        &self,
        room_id: RoomId,
        actor: UserId,
        accept: bool,
    ) -> Result<InteractiveSession, AppError> {
        let mut guard = self.sessions.write().await;
        let session = guard
            .values_mut()
            .find(|s| {
                s.room_id == room_id
                    && s.invitee_id == actor
                    && s.status == InteractiveStatus::Invited
            })
            .ok_or_else(|| AppError::not_found("no pending invite"))?;
        if accept {
            session.accept(actor).map_err(map_interactive)?;
        } else {
            session.decline(actor).map_err(map_interactive)?;
        }
        Ok(session.clone())
    }

    pub async fn leave(
        &self,
        room_id: RoomId,
        actor: UserId,
    ) -> Result<InteractiveSession, AppError> {
        let mut guard = self.sessions.write().await;
        let session = guard
            .values_mut()
            .find(|s| s.room_id == room_id && s.status.is_open() && (s.host_id == actor || s.invitee_id == actor))
            .ok_or_else(|| AppError::not_found("no active session"))?;
        session.end(actor).map_err(map_interactive)?;
        Ok(session.clone())
    }

    pub async fn list_for_room(&self, room_id: RoomId) -> Vec<InteractiveSession> {
        let guard = self.sessions.read().await;
        let mut items: Vec<_> = guard
            .values()
            .filter(|s| s.room_id == room_id)
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    /// Start PK between two rooms. Caller must verify both live + ownership.
    pub async fn start_pk(
        &self,
        room_a_id: RoomId,
        host_a_id: UserId,
        room_b_id: RoomId,
        host_b_id: UserId,
        duration_secs: Option<i64>,
    ) -> Result<PkSession, AppError> {
        let duration = duration_secs.unwrap_or(DEFAULT_PK_DURATION_SECS);
        let mut by_room = self.pk_by_room.write().await;
        let mut pks = self.pks.write().await;

        // Expire any stale PK first for these rooms.
        for rid in [room_a_id, room_b_id] {
            if let Some(id) = by_room.get(&rid).copied() {
                if let Some(pk) = pks.get_mut(&id) {
                    if pk.maybe_expire() {
                        by_room.remove(&pk.room_a_id);
                        by_room.remove(&pk.room_b_id);
                    }
                }
            }
        }

        if by_room.contains_key(&room_a_id) || by_room.contains_key(&room_b_id) {
            return Err(map_pk(PkError::AlreadyInPk));
        }

        let pk = PkSession::start(room_a_id, host_a_id, room_b_id, host_b_id, duration)
            .map_err(map_pk)?;
        by_room.insert(room_a_id, pk.id);
        by_room.insert(room_b_id, pk.id);
        pks.insert(pk.id, pk.clone());
        Ok(pk)
    }

    pub async fn get_pk_for_room(&self, room_id: RoomId) -> Option<PkSession> {
        let by_room = self.pk_by_room.read().await;
        let id = by_room.get(&room_id).copied()?;
        let mut pks = self.pks.write().await;
        let pk = pks.get_mut(&id)?;
        if pk.maybe_expire() {
            // Clear room index for expired PK
            let pk = pk.clone();
            drop(pks);
            let mut by_room = self.pk_by_room.write().await;
            by_room.remove(&pk.room_a_id);
            by_room.remove(&pk.room_b_id);
            return Some(pk);
        }
        Some(pk.clone())
    }

    pub async fn end_pk(&self, room_id: RoomId, actor: UserId) -> Result<PkSession, AppError> {
        let by_room = self.pk_by_room.read().await;
        let id = by_room
            .get(&room_id)
            .copied()
            .ok_or_else(|| AppError::not_found("no active PK"))?;
        drop(by_room);
        let mut pks = self.pks.write().await;
        let pk = pks
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("no active PK"))?;
        let _ = pk.maybe_expire();
        if pk.status != anylive_domain::PkStatus::Active {
            return Err(AppError::not_found("no active PK"));
        }
        if !pk.is_host(actor) {
            return Err(AppError::new(ErrorCode::Forbidden, "only PK hosts may end"));
        }
        pk.end().map_err(map_pk)?;
        let out = pk.clone();
        drop(pks);
        let mut by_room = self.pk_by_room.write().await;
        by_room.remove(&out.room_a_id);
        by_room.remove(&out.room_b_id);
        Ok(out)
    }

    /// Credit PK score for a room when a gift is sent (no-op if room not in PK).
    pub async fn add_gift_score(&self, room_id: RoomId, coins: i64) -> Option<PkSession> {
        let by_room = self.pk_by_room.read().await;
        let id = by_room.get(&room_id).copied()?;
        drop(by_room);
        let mut pks = self.pks.write().await;
        let pk = pks.get_mut(&id)?;
        if pk.maybe_expire() {
            let out = pk.clone();
            drop(pks);
            let mut by_room = self.pk_by_room.write().await;
            by_room.remove(&out.room_a_id);
            by_room.remove(&out.room_b_id);
            return Some(out);
        }
        if pk.add_score(room_id, coins).is_ok() {
            return Some(pk.clone());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invite_accept_flow() {
        let store = InteractiveStore::new();
        let room = RoomId::new();
        let host = UserId::new();
        let guest = UserId::new();
        let s = store.invite(room, host, guest).await.unwrap();
        assert_eq!(s.status, InteractiveStatus::Invited);
        let s = store.respond(room, guest, true).await.unwrap();
        assert_eq!(s.status, InteractiveStatus::Active);
        let s = store.leave(room, guest).await.unwrap();
        assert_eq!(s.status, InteractiveStatus::Ended);
    }

    #[tokio::test]
    async fn seat_occupied() {
        let store = InteractiveStore::new();
        let room = RoomId::new();
        let host = UserId::new();
        store
            .invite(room, host, UserId::new())
            .await
            .unwrap();
        let err = store
            .invite(room, host, UserId::new())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
    }

    #[tokio::test]
    async fn pk_start_score_end() {
        let store = InteractiveStore::new();
        let a = RoomId::new();
        let b = RoomId::new();
        let ha = UserId::new();
        let hb = UserId::new();
        let pk = store
            .start_pk(a, ha, b, hb, Some(60))
            .await
            .unwrap();
        assert_eq!(pk.score_a, 0);
        let updated = store.add_gift_score(a, 50).await.unwrap();
        assert_eq!(updated.score_a, 50);
        let ended = store.end_pk(a, ha).await.unwrap();
        assert_eq!(ended.winner_room_id, Some(a));
        assert!(store.get_pk_for_room(a).await.is_none() || {
            // after end, room index cleared
            store.get_pk_for_room(a).await.map(|p| p.status) != Some(anylive_domain::PkStatus::Active)
        });
    }
}
