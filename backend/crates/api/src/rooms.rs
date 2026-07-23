//! In-memory room store (control plane) + dual backend with Postgres.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_db::{map_room_error, PostgresRoomStore, PgPool};
use anylive_domain::room::RoomError;
use anylive_domain::{Room, RoomId, RoomStatus, UserId};
use tokio::sync::RwLock;

/// Thread-safe in-memory room repository for local/dev and tests.
#[derive(Clone, Default)]
pub struct MemoryRoomStore {
    inner: Arc<RwLock<HashMap<RoomId, Room>>>,
}

impl MemoryRoomStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create(&self, owner_id: UserId, title: impl Into<String>) -> Result<Room, AppError> {
        let room = Room::new(owner_id, title).map_err(map_room_error)?;
        let mut guard = self.inner.write().await;
        guard.insert(room.id, room.clone());
        Ok(room)
    }

    pub async fn get(&self, id: RoomId) -> Result<Room, AppError> {
        self.inner
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::not_found("room not found"))
    }

    pub async fn list(&self, status: Option<RoomStatus>) -> Vec<Room> {
        let guard = self.inner.read().await;
        let mut items: Vec<Room> = guard
            .values()
            .filter(|r| status.map(|s| r.status == s).unwrap_or(true))
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    /// Case-insensitive substring match on title.
    pub async fn search_title(&self, q: &str, limit: usize) -> Vec<Room> {
        let needle = q.trim().to_ascii_lowercase();
        if needle.is_empty() || limit == 0 {
            return Vec::new();
        }
        let guard = self.inner.read().await;
        let mut items: Vec<Room> = guard
            .values()
            .filter(|r| r.title.to_ascii_lowercase().contains(&needle))
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.into_iter().take(limit).collect()
    }

    /// Rooms owned by a user (any status unless filtered).
    pub async fn list_by_owner(&self, owner_id: UserId, status: Option<RoomStatus>) -> Vec<Room> {
        let guard = self.inner.read().await;
        let mut items: Vec<Room> = guard
            .values()
            .filter(|r| r.owner_id == owner_id)
            .filter(|r| status.map(|s| r.status == s).unwrap_or(true))
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    /// Owner-only: Idle -> Live.
    pub async fn start(&self, id: RoomId, actor: UserId) -> Result<Room, AppError> {
        self.mutate(id, actor, true, |room| room.transition(RoomStatus::Live))
            .await
    }

    /// Owner-only: Live -> Idle.
    pub async fn stop(&self, id: RoomId, actor: UserId) -> Result<Room, AppError> {
        self.mutate(id, actor, true, |room| room.transition(RoomStatus::Idle))
            .await
    }

    /// Force close (Idle|Live -> Closed). Owner check optional for admin use.
    pub async fn force_close(
        &self,
        id: RoomId,
        actor: Option<UserId>,
    ) -> Result<Room, AppError> {
        let mut guard = self.inner.write().await;
        let room = guard
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("room not found"))?;
        if let Some(uid) = actor {
            if room.owner_id != uid {
                return Err(AppError::new(ErrorCode::Forbidden, "not room owner"));
            }
        }
        room.transition(RoomStatus::Closed)
            .map_err(map_room_error)?;
        Ok(room.clone())
    }

    async fn mutate<F>(
        &self,
        id: RoomId,
        actor: UserId,
        require_owner: bool,
        f: F,
    ) -> Result<Room, AppError>
    where
        F: FnOnce(&mut Room) -> Result<(), RoomError>,
    {
        let mut guard = self.inner.write().await;
        let room = guard
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("room not found"))?;
        if require_owner && room.owner_id != actor {
            return Err(AppError::new(ErrorCode::Forbidden, "not room owner"));
        }
        f(room).map_err(map_room_error)?;
        Ok(room.clone())
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyRoomStore {
    Memory(MemoryRoomStore),
    Postgres(PostgresRoomStore),
}

impl AnyRoomStore {
    pub fn memory() -> Self {
        Self::Memory(MemoryRoomStore::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresRoomStore::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn create(
        &self,
        owner_id: UserId,
        title: impl Into<String>,
    ) -> Result<Room, AppError> {
        match self {
            Self::Memory(s) => s.create(owner_id, title).await,
            Self::Postgres(s) => s.create(owner_id, title).await,
        }
    }

    pub async fn get(&self, id: RoomId) -> Result<Room, AppError> {
        match self {
            Self::Memory(s) => s.get(id).await,
            Self::Postgres(s) => s.get(id).await,
        }
    }

    pub async fn list(&self, status: Option<RoomStatus>) -> Vec<Room> {
        match self {
            Self::Memory(s) => s.list(status).await,
            Self::Postgres(s) => s.list(status).await,
        }
    }

    pub async fn search_title(&self, q: &str, limit: usize) -> Vec<Room> {
        match self {
            Self::Memory(s) => s.search_title(q, limit).await,
            Self::Postgres(s) => s.search_title(q, limit).await,
        }
    }

    pub async fn list_by_owner(&self, owner_id: UserId, status: Option<RoomStatus>) -> Vec<Room> {
        match self {
            Self::Memory(s) => s.list_by_owner(owner_id, status).await,
            Self::Postgres(s) => s.list_by_owner(owner_id, status).await,
        }
    }

    pub async fn start(&self, id: RoomId, actor: UserId) -> Result<Room, AppError> {
        match self {
            Self::Memory(s) => s.start(id, actor).await,
            Self::Postgres(s) => s.start(id, actor).await,
        }
    }

    pub async fn stop(&self, id: RoomId, actor: UserId) -> Result<Room, AppError> {
        match self {
            Self::Memory(s) => s.stop(id, actor).await,
            Self::Postgres(s) => s.stop(id, actor).await,
        }
    }

    pub async fn force_close(
        &self,
        id: RoomId,
        actor: Option<UserId>,
    ) -> Result<Room, AppError> {
        match self {
            Self::Memory(s) => s.force_close(id, actor).await,
            Self::Postgres(s) => s.force_close(id, actor).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_get_list_start_stop() {
        let store = MemoryRoomStore::new();
        let owner = UserId::new();
        let other = UserId::new();
        let room = store.create(owner, " show ").await.unwrap();
        assert_eq!(room.title, "show");
        assert_eq!(room.status, RoomStatus::Idle);

        let got = store.get(room.id).await.unwrap();
        assert_eq!(got.id, room.id);

        let listed = store.list(Some(RoomStatus::Idle)).await;
        assert_eq!(listed.len(), 1);

        let err = store.start(room.id, other).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);

        let live = store.start(room.id, owner).await.unwrap();
        assert_eq!(live.status, RoomStatus::Live);
        assert_eq!(store.list(Some(RoomStatus::Live)).await.len(), 1);

        let idle = store.stop(room.id, owner).await.unwrap();
        assert_eq!(idle.status, RoomStatus::Idle);

        let closed = store.force_close(room.id, Some(owner)).await.unwrap();
        assert_eq!(closed.status, RoomStatus::Closed);

        let err = store.start(room.id, owner).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
    }
}
