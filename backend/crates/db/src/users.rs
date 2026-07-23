//! [`UserStore`] adapters: Postgres + dual backend for API wiring.

use anylive_auth::{InMemoryUserStore, UserStore};
use anylive_common::{AppError, ErrorCode};
use anylive_domain::{User, UserId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Postgres-backed [`UserStore`] (users table from `001_init.sql`).
#[derive(Clone)]
pub struct PostgresUserStore {
    pool: PgPool,
}

impl PostgresUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    display_name: String,
    email: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: UserId(row.id),
            display_name: row.display_name,
            email: row.email,
            created_at: row.created_at,
        }
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres user store error");
    AppError::new(ErrorCode::Internal, "database error")
}

fn display_name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("user");
    if local.is_empty() {
        "user".to_string()
    } else if local.len() > 64 {
        local[..64].to_string()
    } else {
        local.to_string()
    }
}

/// Lowercase + trim so UNIQUE(email) matches auth OTP normalization.
fn normalize_email_key(email: &str) -> String {
    email.trim().to_lowercase()
}

#[async_trait]
impl UserStore for PostgresUserStore {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let email = normalize_email_key(email);
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, display_name, email, created_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(Into::into))
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, display_name, email, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(Into::into))
    }

    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError> {
        let email = normalize_email_key(email);
        if email.is_empty() || !email.contains('@') {
            return Err(AppError::validation("invalid email"));
        }
        let display = display_name_from_email(&email);
        // UNIQUE(email): concurrent first-login races collapse to one row.
        // DO UPDATE is a no-op touch so RETURNING always yields the existing row.
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (display_name, email)
            VALUES ($1, $2)
            ON CONFLICT (email) DO UPDATE
                SET email = EXCLUDED.email
            RETURNING id, display_name, email, created_at
            "#,
        )
        .bind(&display)
        .bind(&email)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.into())
    }

    async fn update_display_name(
        &self,
        id: UserId,
        display_name: String,
    ) -> Result<User, AppError> {
        let name = User::validate_display_name(display_name)
            .map_err(|e| AppError::validation(format!("{e}")))?;
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET display_name = $2
            WHERE id = $1
            RETURNING id, display_name, email, created_at
            "#,
        )
        .bind(id.0)
        .bind(&name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?
        .ok_or_else(|| AppError::not_found("user not found"))?;
        Ok(row.into())
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyUserStore {
    Memory(InMemoryUserStore),
    Postgres(PostgresUserStore),
}

impl AnyUserStore {
    pub fn memory() -> Self {
        Self::Memory(InMemoryUserStore::default())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresUserStore::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    /// Case-insensitive substring search on display_name.
    pub async fn search_display_name(
        &self,
        q: &str,
        limit: usize,
    ) -> Result<Vec<User>, AppError> {
        let needle = q.trim();
        if needle.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        match self {
            Self::Memory(s) => Ok(s.search_display_name(needle, limit).await),
            Self::Postgres(s) => s.search_display_name(needle, limit).await,
        }
    }
}

impl PostgresUserStore {
    pub async fn search_display_name(
        &self,
        q: &str,
        limit: usize,
    ) -> Result<Vec<User>, AppError> {
        let pattern = format!("%{}%", q.trim());
        let rows = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, display_name, email, created_at
            FROM users
            WHERE display_name ILIKE $1
            ORDER BY display_name ASC
            LIMIT $2
            "#,
        )
        .bind(&pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl UserStore for AnyUserStore {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        match self {
            Self::Memory(s) => s.find_by_email(email).await,
            Self::Postgres(s) => s.find_by_email(email).await,
        }
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        match self {
            Self::Memory(s) => s.find_by_id(id).await,
            Self::Postgres(s) => s.find_by_id(id).await,
        }
    }

    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError> {
        match self {
            Self::Memory(s) => s.upsert_by_email(email).await,
            Self::Postgres(s) => s.upsert_by_email(email).await,
        }
    }

    async fn update_display_name(
        &self,
        id: UserId,
        display_name: String,
    ) -> Result<User, AppError> {
        match self {
            Self::Memory(s) => s.update_display_name(id, display_name).await,
            Self::Postgres(s) => s.update_display_name(id, display_name).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;

    #[tokio::test]
    async fn memory_backend_upsert_idempotent() {
        let store = AnyUserStore::memory();
        let a = store.upsert_by_email("a@example.com").await.unwrap();
        let b = store.upsert_by_email("a@example.com").await.unwrap();
        assert_eq!(a.id, b.id);
        assert!(!store.is_postgres());
    }

    #[tokio::test]
    async fn memory_backend_update_display_name() {
        let store = AnyUserStore::memory();
        let u = store.upsert_by_email("rename@example.com").await.unwrap();
        let updated = store
            .update_display_name(u.id, "Renamed".into())
            .await
            .unwrap();
        assert_eq!(updated.display_name, "Renamed");
        let again = store.find_by_id(u.id).await.unwrap().unwrap();
        assert_eq!(again.display_name, "Renamed");
    }

    #[test]
    fn email_key_is_trimmed_and_lowercased() {
        assert_eq!(normalize_email_key("  Alice@Example.COM "), "alice@example.com");
        assert_eq!(display_name_from_email("alice@example.com"), "alice");
        assert_eq!(display_name_from_email("@example.com"), "user");
    }

    /// Optional integration: runs only when `USE_POSTGRES=1` and `DATABASE_URL` are set.
    ///
    /// ```text
    /// USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
    ///   cargo test -p anylive-db postgres_user_store_roundtrip -- --nocapture
    /// ```
    #[tokio::test]
    async fn postgres_user_store_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");
        let store = PostgresUserStore::new(pool);

        let email = format!("pg-test-{}@example.com", Uuid::new_v4());
        let a = store.upsert_by_email(&email).await.expect("upsert");
        let b = store.upsert_by_email(&email).await.expect("upsert again");
        assert_eq!(a.id, b.id);
        assert_eq!(a.email.as_deref(), Some(email.as_str()));

        let by_email = store.find_by_email(&email).await.unwrap().unwrap();
        assert_eq!(by_email.id, a.id);
        let by_id = store.find_by_id(a.id).await.unwrap().unwrap();
        assert_eq!(by_id.email.as_deref(), Some(email.as_str()));
    }
}
