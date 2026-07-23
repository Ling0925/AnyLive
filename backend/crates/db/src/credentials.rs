//! Credential store: Postgres + dual backend.

use anylive_auth::{CredentialRecord, CredentialStore, InMemoryCredentialStore};
use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresCredentialStore {
    pool: PgPool,
}

impl PostgresCredentialStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CredRow {
    user_id: Uuid,
    password_hash: String,
    password_updated_at: DateTime<Utc>,
    must_change_password: bool,
    failed_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
}

impl From<CredRow> for CredentialRecord {
    fn from(r: CredRow) -> Self {
        Self {
            user_id: UserId(r.user_id),
            password_hash: r.password_hash,
            password_updated_at: r.password_updated_at,
            must_change_password: r.must_change_password,
            failed_attempts: r.failed_attempts.max(0) as u32,
            locked_until: r.locked_until,
        }
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres credential store error");
    AppError::new(ErrorCode::Internal, "database error")
}

#[async_trait]
impl CredentialStore for PostgresCredentialStore {
    async fn get(&self, user_id: UserId) -> Result<Option<CredentialRecord>, AppError> {
        let row = sqlx::query_as::<_, CredRow>(
            r#"
            SELECT user_id, password_hash, password_updated_at, must_change_password,
                   failed_attempts, locked_until
            FROM user_credentials
            WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(Into::into))
    }

    async fn upsert(&self, record: CredentialRecord) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO user_credentials (
                user_id, password_hash, password_updated_at, must_change_password,
                failed_attempts, locked_until
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id) DO UPDATE SET
                password_hash = EXCLUDED.password_hash,
                password_updated_at = EXCLUDED.password_updated_at,
                must_change_password = EXCLUDED.must_change_password,
                failed_attempts = EXCLUDED.failed_attempts,
                locked_until = EXCLUDED.locked_until
            "#,
        )
        .bind(record.user_id.0)
        .bind(&record.password_hash)
        .bind(record.password_updated_at)
        .bind(record.must_change_password)
        .bind(record.failed_attempts as i32)
        .bind(record.locked_until)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(())
    }

    async fn delete(&self, user_id: UserId) -> Result<(), AppError> {
        sqlx::query("DELETE FROM user_credentials WHERE user_id = $1")
            .bind(user_id.0)
            .execute(&self.pool)
            .await
            .map_err(map_db)?;
        Ok(())
    }
}

#[derive(Clone)]
pub enum AnyCredentialStore {
    Memory(InMemoryCredentialStore),
    Postgres(PostgresCredentialStore),
}

impl AnyCredentialStore {
    pub fn memory() -> Self {
        Self::Memory(InMemoryCredentialStore::default())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresCredentialStore::new(pool))
    }
}

#[async_trait]
impl CredentialStore for AnyCredentialStore {
    async fn get(&self, user_id: UserId) -> Result<Option<CredentialRecord>, AppError> {
        match self {
            Self::Memory(s) => s.get(user_id).await,
            Self::Postgres(s) => s.get(user_id).await,
        }
    }

    async fn upsert(&self, record: CredentialRecord) -> Result<(), AppError> {
        match self {
            Self::Memory(s) => s.upsert(record).await,
            Self::Postgres(s) => s.upsert(record).await,
        }
    }

    async fn delete(&self, user_id: UserId) -> Result<(), AppError> {
        match self {
            Self::Memory(s) => s.delete(user_id).await,
            Self::Postgres(s) => s.delete(user_id).await,
        }
    }
}
