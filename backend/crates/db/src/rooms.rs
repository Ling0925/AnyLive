//! [`PostgresRoomStore`] + dual [`AnyRoomStore`] matching the API `MemoryRoomStore` surface.
//!
//! Offline unit tests never open Postgres. Integration runs when `USE_POSTGRES=1`.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::room::RoomError;
use anylive_domain::{Room, RoomId, RoomStatus, UserId};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Postgres-backed room repository (`rooms` table from `001_init.sql`).
#[derive(Clone)]
pub struct PostgresRoomStore {
    pool: PgPool,
}

impl PostgresRoomStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn create(
        &self,
        owner_id: UserId,
        title: impl Into<String>,
    ) -> Result<Room, AppError> {
        let room = Room::new(owner_id, title).map_err(map_room_error)?;
        let row = sqlx::query_as::<_, RoomRow>(
            r#"
            INSERT INTO rooms (id, owner_id, title, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, owner_id, title, status, created_at, updated_at
            "#,
        )
        .bind(room.id.0)
        .bind(room.owner_id.0)
        .bind(&room.title)
        .bind(room.status.as_str())
        .bind(room.created_at)
        .bind(room.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        row.into_room()
    }

    pub async fn get(&self, id: RoomId) -> Result<Room, AppError> {
        let row = sqlx::query_as::<_, RoomRow>(
            r#"
            SELECT id, owner_id, title, status, created_at, updated_at
            FROM rooms
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        match row {
            Some(r) => r.into_room(),
            None => Err(AppError::not_found("room not found")),
        }
    }

    pub async fn list(&self, status: Option<RoomStatus>) -> Vec<Room> {
        let status_str = status.map(|s| s.as_str().to_string());
        let rows = sqlx::query_as::<_, RoomRow>(
            r#"
            SELECT id, owner_id, title, status, created_at, updated_at
            FROM rooms
            WHERE ($1::text IS NULL OR status = $1)
            ORDER BY created_at DESC
            "#,
        )
        .bind(status_str)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "postgres room list failed");
            Vec::new()
        });
        rows.into_iter()
            .filter_map(|r| r.into_room().ok())
            .collect()
    }

    /// Owner-only: Idle -> Live.
    pub async fn start(&self, id: RoomId, actor: UserId) -> Result<Room, AppError> {
        self.transition(id, Some(actor), RoomStatus::Live).await
    }

    /// Owner-only: Live -> Idle.
    pub async fn stop(&self, id: RoomId, actor: UserId) -> Result<Room, AppError> {
        self.transition(id, Some(actor), RoomStatus::Idle).await
    }

    /// Force close (Idle|Live -> Closed). Owner check when `actor` is `Some`.
    pub async fn force_close(
        &self,
        id: RoomId,
        actor: Option<UserId>,
    ) -> Result<Room, AppError> {
        self.transition(id, actor, RoomStatus::Closed).await
    }

    async fn transition(
        &self,
        id: RoomId,
        actor: Option<UserId>,
        next: RoomStatus,
    ) -> Result<Room, AppError> {
        let current = self.get(id).await?;
        if let Some(uid) = actor {
            if current.owner_id != uid {
                return Err(AppError::new(ErrorCode::Forbidden, "not room owner"));
            }
        }
        if !current.status.can_transition_to(next) {
            return Err(map_room_error(RoomError::InvalidTransition {
                from: current.status,
                to: next,
            }));
        }

        // Conditional update prevents lost races on concurrent transitions.
        let row = sqlx::query_as::<_, RoomRow>(
            r#"
            UPDATE rooms
            SET status = $2, updated_at = now()
            WHERE id = $1 AND status = $3
            RETURNING id, owner_id, title, status, created_at, updated_at
            "#,
        )
        .bind(id.0)
        .bind(next.as_str())
        .bind(current.status.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;

        match row {
            Some(r) => r.into_room(),
            None => {
                // Re-read to surface a precise conflict / not-found.
                let again = self.get(id).await?;
                Err(map_room_error(RoomError::InvalidTransition {
                    from: again.status,
                    to: next,
                }))
            }
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RoomRow {
    id: Uuid,
    owner_id: Uuid,
    title: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RoomRow {
    fn into_room(self) -> Result<Room, AppError> {
        let status = RoomStatus::parse(&self.status).ok_or_else(|| {
            AppError::new(
                ErrorCode::Internal,
                format!("invalid room status in db: {}", self.status),
            )
        })?;
        Ok(Room {
            id: RoomId(self.id),
            owner_id: UserId(self.owner_id),
            title: self.title,
            status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres room store error");
    // FK violation on owner_id → treat as validation/not-found for the owner.
    if let sqlx::Error::Database(db) = &err {
        if db.constraint() == Some("rooms_owner_id_fkey") {
            return AppError::validation("owner user does not exist");
        }
    }
    AppError::new(ErrorCode::Internal, "database error")
}

pub fn map_room_error(err: RoomError) -> AppError {
    match err {
        RoomError::InvalidTitle => AppError::validation("invalid title"),
        RoomError::InvalidTransition { from, to } => AppError::new(
            ErrorCode::Conflict,
            format!(
                "invalid room transition from {} to {}",
                from.as_str(),
                to.as_str()
            ),
        ),
    }
}

/// Pure SQL fragments (offline-testable, no live DB).
#[allow(dead_code)]
pub mod sql {
    /// Insert returning full room row.
    pub const INSERT_ROOM: &str = r#"
            INSERT INTO rooms (id, owner_id, title, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, owner_id, title, status, created_at, updated_at
            "#;

    pub const SELECT_BY_ID: &str = r#"
            SELECT id, owner_id, title, status, created_at, updated_at
            FROM rooms
            WHERE id = $1
            "#;

    pub const LIST_BY_STATUS: &str = r#"
            SELECT id, owner_id, title, status, created_at, updated_at
            FROM rooms
            WHERE ($1::text IS NULL OR status = $1)
            ORDER BY created_at DESC
            "#;

    pub const UPDATE_STATUS: &str = r#"
            UPDATE rooms
            SET status = $2, updated_at = now()
            WHERE id = $1 AND status = $3
            RETURNING id, owner_id, title, status, created_at, updated_at
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[test]
    fn sql_fragments_mention_rooms_table() {
        assert!(sql::INSERT_ROOM.contains("INSERT INTO rooms"));
        assert!(sql::SELECT_BY_ID.contains("WHERE id = $1"));
        assert!(sql::LIST_BY_STATUS.contains("ORDER BY created_at DESC"));
        assert!(sql::UPDATE_STATUS.contains("status = $3"));
    }

    #[test]
    fn map_room_error_conflict() {
        let err = map_room_error(RoomError::InvalidTransition {
            from: RoomStatus::Closed,
            to: RoomStatus::Live,
        });
        assert_eq!(err.code, ErrorCode::Conflict);
    }

    #[test]
    fn map_room_error_title() {
        let err = map_room_error(RoomError::InvalidTitle);
        assert_eq!(err.code, ErrorCode::Validation);
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_room_store_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        // Ensure owner exists (FK).
        let owner_email = format!("room-owner-{}@example.com", Uuid::new_v4());
        let owner = crate::PostgresUserStore::new(pool.clone())
            .upsert_by_email(&owner_email)
            .await
            .expect("owner");
        let other = crate::PostgresUserStore::new(pool.clone())
            .upsert_by_email(&format!("room-other-{}@example.com", Uuid::new_v4()))
            .await
            .expect("other");

        let store = PostgresRoomStore::new(pool);
        let room = store.create(owner.id, "  live show ").await.unwrap();
        assert_eq!(room.title, "live show");
        assert_eq!(room.status, RoomStatus::Idle);

        let got = store.get(room.id).await.unwrap();
        assert_eq!(got.id, room.id);

        let listed = store.list(Some(RoomStatus::Idle)).await;
        assert!(listed.iter().any(|r| r.id == room.id));

        let err = store.start(room.id, other.id).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);

        let live = store.start(room.id, owner.id).await.unwrap();
        assert_eq!(live.status, RoomStatus::Live);

        let idle = store.stop(room.id, owner.id).await.unwrap();
        assert_eq!(idle.status, RoomStatus::Idle);

        let closed = store.force_close(room.id, None).await.unwrap();
        assert_eq!(closed.status, RoomStatus::Closed);

        let err = store.start(room.id, owner.id).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
    }
}
