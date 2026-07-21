//! [`PostgresChat`] + dual [`AnyChat`] matching [`MemoryChatBus`] surface.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, UserId};
use anylive_realtime::{ChatMessage, MemoryChatBus, MAX_CHAT_LEN};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Validate chat body the same way as [`MemoryChatBus::post`].
pub fn validate_chat_body(body: &str) -> Result<String, AppError> {
    let body = body.trim().to_string();
    if body.is_empty() || body.len() > MAX_CHAT_LEN {
        return Err(AppError::validation("invalid chat body"));
    }
    // basic spam: reject control chars except newline
    if body.chars().any(|c| c.is_control() && c != '\n') {
        return Err(AppError::validation("chat body has control characters"));
    }
    Ok(body)
}

/// Postgres-backed chat history (`chat_messages` table from `001_init.sql`).
#[derive(Clone)]
pub struct PostgresChat {
    pool: PgPool,
}

impl PostgresChat {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn post(
        &self,
        room_id: RoomId,
        sender_id: UserId,
        sender_name: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<ChatMessage, AppError> {
        let body = validate_chat_body(&body.into())?;
        let sender_name = sender_name.into();
        let id = Uuid::new_v4();
        let created_at = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO chat_messages (id, room_id, sender_id, sender_name, body, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(room_id.0)
        .bind(sender_id.0)
        .bind(&sender_name)
        .bind(&body)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;

        Ok(ChatMessage {
            id,
            room_id,
            sender_id,
            sender_name,
            body,
            created_at,
        })
    }

    pub async fn recent(&self, room_id: RoomId, limit: usize) -> Vec<ChatMessage> {
        let limit = limit.clamp(1, 100) as i64;
        let rows = sqlx::query_as::<_, ChatRow>(
            r#"
            SELECT id, room_id, sender_id, sender_name, body, created_at
            FROM (
                SELECT id, room_id, sender_id, sender_name, body, created_at
                FROM chat_messages
                WHERE room_id = $1
                ORDER BY created_at DESC
                LIMIT $2
            ) t
            ORDER BY created_at ASC
            "#,
        )
        .bind(room_id.0)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "postgres chat recent failed");
            Vec::new()
        });

        rows.into_iter().map(ChatMessage::from).collect()
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyChat {
    Memory(MemoryChatBus),
    Postgres(PostgresChat),
}

impl AnyChat {
    pub fn memory() -> Self {
        Self::Memory(MemoryChatBus::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresChat::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn post(
        &self,
        room_id: RoomId,
        sender_id: UserId,
        sender_name: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<ChatMessage, AppError> {
        match self {
            Self::Memory(c) => c.post(room_id, sender_id, sender_name, body).await,
            Self::Postgres(c) => c.post(room_id, sender_id, sender_name, body).await,
        }
    }

    pub async fn recent(&self, room_id: RoomId, limit: usize) -> Vec<ChatMessage> {
        match self {
            Self::Memory(c) => c.recent(room_id, limit).await,
            Self::Postgres(c) => c.recent(room_id, limit).await,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ChatRow {
    id: Uuid,
    room_id: Uuid,
    sender_id: Uuid,
    sender_name: String,
    body: String,
    created_at: DateTime<Utc>,
}

impl From<ChatRow> for ChatMessage {
    fn from(r: ChatRow) -> Self {
        Self {
            id: r.id,
            room_id: RoomId(r.room_id),
            sender_id: UserId(r.sender_id),
            sender_name: r.sender_name,
            body: r.body,
            created_at: r.created_at,
        }
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres chat store error");
    AppError::new(ErrorCode::Internal, "database error")
}

/// Pure SQL fragments (offline-testable, no live DB).
#[allow(dead_code)]
pub mod sql {
    pub const INSERT_MESSAGE: &str = r#"
            INSERT INTO chat_messages (id, room_id, sender_id, sender_name, body, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#;

    pub const SELECT_RECENT: &str = r#"
            SELECT id, room_id, sender_id, sender_name, body, created_at
            FROM (
                SELECT id, room_id, sender_id, sender_name, body, created_at
                FROM chat_messages
                WHERE room_id = $1
                ORDER BY created_at DESC
                LIMIT $2
            ) t
            ORDER BY created_at ASC
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_common::ErrorCode;

    #[test]
    fn sql_fragments_mention_chat_messages() {
        assert!(sql::INSERT_MESSAGE.contains("INSERT INTO chat_messages"));
        assert!(sql::SELECT_RECENT.contains("ORDER BY created_at DESC"));
        assert!(sql::SELECT_RECENT.contains("ORDER BY created_at ASC"));
        assert!(sql::SELECT_RECENT.contains("LIMIT $2"));
    }

    #[test]
    fn validate_rejects_empty_and_whitespace() {
        assert!(validate_chat_body("").is_err());
        assert!(validate_chat_body("   ").is_err());
        assert!(validate_chat_body("\n\t").is_err());
    }

    #[test]
    fn validate_rejects_too_long() {
        let long = "a".repeat(MAX_CHAT_LEN + 1);
        let err = validate_chat_body(&long).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn validate_rejects_control_chars() {
        let err = validate_chat_body("hello\u{0001}world").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(err.message.contains("control"));
    }

    #[test]
    fn validate_allows_newline_and_max_len() {
        let ok = validate_chat_body("line1\nline2").unwrap();
        assert_eq!(ok, "line1\nline2");
        let max = "b".repeat(MAX_CHAT_LEN);
        assert_eq!(validate_chat_body(&max).unwrap().len(), MAX_CHAT_LEN);
    }

    #[tokio::test]
    async fn memory_backend_post_and_recent() {
        let chat = AnyChat::memory();
        let room = RoomId::new();
        let user = UserId::new();
        chat.post(room, user, "Alice", "hello").await.unwrap();
        chat.post(room, user, "Alice", "world").await.unwrap();
        let recent = chat.recent(room, 10).await;
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].body, "hello");
        assert_eq!(recent[1].body, "world");
        assert!(!chat.is_postgres());
    }

    #[tokio::test]
    async fn memory_backend_reject_empty() {
        let chat = AnyChat::memory();
        let err = chat
            .post(RoomId::new(), UserId::new(), "x", "   ")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[tokio::test]
    async fn memory_backend_reject_control_chars() {
        let chat = AnyChat::memory();
        let err = chat
            .post(RoomId::new(), UserId::new(), "x", "bad\0body")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[tokio::test]
    async fn memory_recent_clamps_limit() {
        let chat = AnyChat::memory();
        let room = RoomId::new();
        let user = UserId::new();
        for i in 0..5 {
            chat.post(room, user, "A", format!("m{i}")).await.unwrap();
        }
        // limit 0 clamps to 1
        assert_eq!(chat.recent(room, 0).await.len(), 1);
        // oversize clamps to 100 but only 5 exist
        assert_eq!(chat.recent(room, 500).await.len(), 5);
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_chat_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let chat = PostgresChat::new(pool);
        let room = RoomId::new();
        let user = UserId::new();
        let m1 = chat
            .post(room, user, "Alice", "hello pg")
            .await
            .expect("post1");
        let m2 = chat
            .post(room, user, "Alice", "world pg")
            .await
            .expect("post2");
        assert_ne!(m1.id, m2.id);

        let recent = chat.recent(room, 10).await;
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].body, "hello pg");
        assert_eq!(recent[1].body, "world pg");
        assert_eq!(recent[0].sender_name, "Alice");

        assert!(chat
            .post(room, user, "Alice", "   ")
            .await
            .is_err());
    }
}
