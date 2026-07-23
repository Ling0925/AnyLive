//! [`UserStore`] adapters: Postgres + dual backend for API wiring.

use anylive_auth::{InMemoryUserStore, UserStore};
use anylive_common::{AppError, ErrorCode};
use anylive_domain::{User, UserId, UserStatus};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Postgres-backed [`UserStore`] (users table from `001_init.sql` + `011_user_credentials.sql`).
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
    username: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: UserId(row.id),
            display_name: row.display_name,
            email: row.email,
            username: row.username,
            status: UserStatus::parse(&row.status).unwrap_or(UserStatus::Active),
            created_at: row.created_at,
        }
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres user store error");
    // Unique violations → conflict for admin create paths.
    if let sqlx::Error::Database(ref db) = err {
        if db.constraint().is_some() {
            return AppError::new(ErrorCode::Conflict, "user identity conflict");
        }
    }
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

const USER_COLS: &str = "id, display_name, email, username, status, created_at";

#[async_trait]
impl UserStore for PostgresUserStore {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let email = normalize_email_key(email);
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLS} FROM users WHERE email = $1"
        ))
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(Into::into))
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLS} FROM users WHERE id = $1"
        ))
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(Into::into))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let username = username.trim().to_ascii_lowercase();
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLS} FROM users WHERE lower(username) = $1"
        ))
        .bind(&username)
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
        let row = sqlx::query_as::<_, UserRow>(&format!(
            r#"
            INSERT INTO users (display_name, email, status)
            VALUES ($1, $2, 'active')
            ON CONFLICT (email) DO UPDATE
                SET email = EXCLUDED.email
            RETURNING {USER_COLS}
            "#
        ))
        .bind(&display)
        .bind(&email)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.into())
    }

    async fn create_user(&self, user: User) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            r#"
            INSERT INTO users (id, display_name, email, username, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING {USER_COLS}
            "#
        ))
        .bind(user.id.0)
        .bind(&user.display_name)
        .bind(&user.email)
        .bind(&user.username)
        .bind(user.status.as_str())
        .bind(user.created_at)
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
        let row = sqlx::query_as::<_, UserRow>(&format!(
            r#"
            UPDATE users
            SET display_name = $2
            WHERE id = $1
            RETURNING {USER_COLS}
            "#
        ))
        .bind(id.0)
        .bind(&name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?
        .ok_or_else(|| AppError::not_found("user not found"))?;
        Ok(row.into())
    }

    async fn update_account(
        &self,
        id: UserId,
        display_name: Option<String>,
        email: Option<Option<String>>,
        username: Option<Option<String>>,
        status: Option<UserStatus>,
    ) -> Result<User, AppError> {
        // Load current, apply patches in app, write back (simple for Wave A).
        let mut user = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("user not found"))?;
        if let Some(name) = display_name {
            user.display_name = User::validate_display_name(name)
                .map_err(|e| AppError::validation(format!("{e}")))?;
        }
        if let Some(status) = status {
            user.status = status;
        }
        if let Some(email_opt) = email {
            user.email = match email_opt {
                Some(e) => {
                    let e = normalize_email_key(&e);
                    if !e.contains('@') {
                        return Err(AppError::validation("invalid email"));
                    }
                    Some(e)
                }
                None => None,
            };
        }
        if let Some(username_opt) = username {
            user.username = match username_opt {
                Some(u) => Some(
                    User::validate_username(u).map_err(|e| AppError::validation(format!("{e}")))?,
                ),
                None => None,
            };
        }
        let row = sqlx::query_as::<_, UserRow>(&format!(
            r#"
            UPDATE users
            SET display_name = $2, email = $3, username = $4, status = $5
            WHERE id = $1
            RETURNING {USER_COLS}
            "#
        ))
        .bind(id.0)
        .bind(&user.display_name)
        .bind(&user.email)
        .bind(&user.username)
        .bind(user.status.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?
        .ok_or_else(|| AppError::not_found("user not found"))?;
        Ok(row.into())
    }

    async fn list_users(
        &self,
        q: Option<&str>,
        status: Option<UserStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<User>, usize), AppError> {
        let limit = limit.clamp(1, 100) as i64;
        let offset = offset as i64;
        let needle = q.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let status_s = status.map(|s| s.as_str().to_string());

        // Two queries: total + page (Wave A simplicity).
        let total: i64 = if let Some(ref n) = needle {
            let pattern = format!("%{n}%");
            if let Some(ref st) = status_s {
                sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM users
                    WHERE status = $1
                      AND (display_name ILIKE $2 OR email ILIKE $2 OR username ILIKE $2)
                    "#,
                )
                .bind(st)
                .bind(&pattern)
                .fetch_one(&self.pool)
                .await
            } else {
                sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM users
                    WHERE display_name ILIKE $1 OR email ILIKE $1 OR username ILIKE $1
                    "#,
                )
                .bind(&pattern)
                .fetch_one(&self.pool)
                .await
            }
        } else if let Some(ref st) = status_s {
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = $1")
                .bind(st)
                .fetch_one(&self.pool)
                .await
        } else {
            sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(&self.pool)
                .await
        }
        .map_err(map_db)?;

        let rows = if let Some(ref n) = needle {
            let pattern = format!("%{n}%");
            if let Some(ref st) = status_s {
                sqlx::query_as::<_, UserRow>(&format!(
                    r#"
                    SELECT {USER_COLS} FROM users
                    WHERE status = $1
                      AND (display_name ILIKE $2 OR email ILIKE $2 OR username ILIKE $2)
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                    "#
                ))
                .bind(st)
                .bind(&pattern)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            } else {
                sqlx::query_as::<_, UserRow>(&format!(
                    r#"
                    SELECT {USER_COLS} FROM users
                    WHERE display_name ILIKE $1 OR email ILIKE $1 OR username ILIKE $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#
                ))
                .bind(&pattern)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        } else if let Some(ref st) = status_s {
            sqlx::query_as::<_, UserRow>(&format!(
                r#"
                SELECT {USER_COLS} FROM users
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#
            ))
            .bind(st)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, UserRow>(&format!(
                r#"
                SELECT {USER_COLS} FROM users
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#
            ))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(map_db)?;

        Ok((
            rows.into_iter().map(Into::into).collect(),
            total as usize,
        ))
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
        let rows = sqlx::query_as::<_, UserRow>(&format!(
            r#"
            SELECT {USER_COLS}
            FROM users
            WHERE display_name ILIKE $1
            ORDER BY display_name ASC
            LIMIT $2
            "#
        ))
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

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        match self {
            Self::Memory(s) => s.find_by_username(username).await,
            Self::Postgres(s) => s.find_by_username(username).await,
        }
    }

    async fn upsert_by_email(&self, email: &str) -> Result<User, AppError> {
        match self {
            Self::Memory(s) => s.upsert_by_email(email).await,
            Self::Postgres(s) => s.upsert_by_email(email).await,
        }
    }

    async fn create_user(&self, user: User) -> Result<User, AppError> {
        match self {
            Self::Memory(s) => s.create_user(user).await,
            Self::Postgres(s) => s.create_user(user).await,
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

    async fn update_account(
        &self,
        id: UserId,
        display_name: Option<String>,
        email: Option<Option<String>>,
        username: Option<Option<String>>,
        status: Option<UserStatus>,
    ) -> Result<User, AppError> {
        match self {
            Self::Memory(s) => {
                s.update_account(id, display_name, email, username, status)
                    .await
            }
            Self::Postgres(s) => {
                s.update_account(id, display_name, email, username, status)
                    .await
            }
        }
    }

    async fn list_users(
        &self,
        q: Option<&str>,
        status: Option<UserStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<User>, usize), AppError> {
        match self {
            Self::Memory(s) => s.list_users(q, status, limit, offset).await,
            Self::Postgres(s) => s.list_users(q, status, limit, offset).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_backend_upsert_idempotent() {
        let store = AnyUserStore::memory();
        let a = store.upsert_by_email("a@example.com").await.unwrap();
        let b = store.upsert_by_email("a@example.com").await.unwrap();
        assert_eq!(a.id, b.id);
        assert!(!store.is_postgres());
    }

    #[tokio::test]
    async fn memory_backend_create_with_username() {
        let store = AnyUserStore::memory();
        let mut u = User::new("Host", Some("h@example.com".into())).unwrap();
        u.username = Some("host1".into());
        let created = store.create_user(u.clone()).await.unwrap();
        let found = store.find_by_username("HOST1").await.unwrap().unwrap();
        assert_eq!(found.id, created.id);
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
    }

    #[test]
    fn email_key_is_trimmed_and_lowercased() {
        assert_eq!(
            normalize_email_key("  Alice@Example.COM "),
            "alice@example.com"
        );
        assert_eq!(display_name_from_email("alice@example.com"), "alice");
    }
}
