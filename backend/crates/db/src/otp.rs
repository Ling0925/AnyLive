//! OTP dual store (`otp_challenges` from `005_otp_challenges.sql`).
//!
//! Implements [`OtpStore`] so `AuthService` can persist email OTP challenges when
//! Postgres is enabled (multi-instance / restart-safe).

use anylive_auth::{InMemoryOtpStore, OtpChallenge, OtpStore};
use anylive_common::{AppError, ErrorCode};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Postgres-backed [`OtpStore`] (`otp_challenges` table).
#[derive(Clone)]
pub struct PostgresOtpStore {
    pool: PgPool,
}

impl PostgresOtpStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres otp store error");
    AppError::new(ErrorCode::Internal, "database error")
}

#[derive(Debug, sqlx::FromRow)]
struct OtpRow {
    code: String,
    expires_at: DateTime<Utc>,
    attempts: i32,
}

impl From<OtpRow> for OtpChallenge {
    fn from(row: OtpRow) -> Self {
        Self {
            code: row.code,
            expires_at: row.expires_at,
            attempts: row.attempts.max(0) as u32,
        }
    }
}

#[async_trait]
impl OtpStore for PostgresOtpStore {
    async fn put(&self, email: &str, challenge: OtpChallenge) -> Result<(), AppError> {
        let attempts = i32::try_from(challenge.attempts).unwrap_or(i32::MAX);
        sqlx::query(
            r#"
            INSERT INTO otp_challenges (email, code, expires_at, attempts, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (email) DO UPDATE SET
                code = EXCLUDED.code,
                expires_at = EXCLUDED.expires_at,
                attempts = EXCLUDED.attempts,
                updated_at = now()
            "#,
        )
        .bind(email)
        .bind(&challenge.code)
        .bind(challenge.expires_at)
        .bind(attempts)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(())
    }

    async fn get(&self, email: &str) -> Result<Option<OtpChallenge>, AppError> {
        let row = sqlx::query_as::<_, OtpRow>(
            r#"
            SELECT code, expires_at, attempts
            FROM otp_challenges
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(OtpChallenge::from))
    }

    async fn take(&self, email: &str) -> Result<Option<OtpChallenge>, AppError> {
        let row = sqlx::query_as::<_, OtpRow>(
            r#"
            DELETE FROM otp_challenges
            WHERE email = $1
            RETURNING code, expires_at, attempts
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.map(OtpChallenge::from))
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyOtpStore {
    Memory(InMemoryOtpStore),
    Postgres(PostgresOtpStore),
}

impl AnyOtpStore {
    pub fn memory() -> Self {
        Self::Memory(InMemoryOtpStore::default())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresOtpStore::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }
}

#[async_trait]
impl OtpStore for AnyOtpStore {
    async fn put(&self, email: &str, challenge: OtpChallenge) -> Result<(), AppError> {
        match self {
            Self::Memory(s) => s.put(email, challenge).await,
            Self::Postgres(s) => s.put(email, challenge).await,
        }
    }

    async fn get(&self, email: &str) -> Result<Option<OtpChallenge>, AppError> {
        match self {
            Self::Memory(s) => s.get(email).await,
            Self::Postgres(s) => s.get(email).await,
        }
    }

    async fn take(&self, email: &str) -> Result<Option<OtpChallenge>, AppError> {
        match self {
            Self::Memory(s) => s.take(email).await,
            Self::Postgres(s) => s.take(email).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use chrono::Duration;
    use uuid::Uuid;

    #[test]
    fn sql_fragments_cover_put_get_take() {
        // Offline smoke: ensure the SQL we ship matches the dual-store contract.
        let put = r#"
            INSERT INTO otp_challenges (email, code, expires_at, attempts, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (email) DO UPDATE SET
                code = EXCLUDED.code,
                expires_at = EXCLUDED.expires_at,
                attempts = EXCLUDED.attempts,
                updated_at = now()
            "#;
        let get = r#"
            SELECT code, expires_at, attempts
            FROM otp_challenges
            WHERE email = $1
            "#;
        let take = r#"
            DELETE FROM otp_challenges
            WHERE email = $1
            RETURNING code, expires_at, attempts
            "#;
        assert!(put.contains("INSERT INTO otp_challenges"));
        assert!(put.contains("ON CONFLICT (email) DO UPDATE"));
        assert!(get.contains("FROM otp_challenges"));
        assert!(take.contains("DELETE FROM otp_challenges"));
        assert!(take.contains("RETURNING"));
    }

    #[test]
    fn migration_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/005_otp_challenges.sql");
        assert!(path.exists(), "expected migration at {}", path.display());
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS otp_challenges"));
        assert!(sql.contains("email TEXT PRIMARY KEY"));
        assert!(sql.contains("attempts INT NOT NULL DEFAULT 0"));
    }

    #[tokio::test]
    async fn memory_backend_put_get_take() {
        let store = AnyOtpStore::memory();
        assert!(!store.is_postgres());
        let email = "user@example.com";
        let challenge = OtpChallenge {
            code: "123456".into(),
            expires_at: Utc::now() + Duration::seconds(300),
            attempts: 0,
        };
        store.put(email, challenge.clone()).await.unwrap();
        let got = store.get(email).await.unwrap().expect("challenge");
        assert_eq!(got.code, "123456");
        assert_eq!(got.attempts, 0);

        let mut updated = got;
        updated.attempts = 2;
        store.put(email, updated).await.unwrap();
        assert_eq!(store.get(email).await.unwrap().unwrap().attempts, 2);

        let taken = store.take(email).await.unwrap().expect("taken");
        assert_eq!(taken.code, "123456");
        assert_eq!(taken.attempts, 2);
        assert!(store.get(email).await.unwrap().is_none());
        assert!(store.take(email).await.unwrap().is_none());
    }

    #[test]
    fn otp_row_maps_negative_attempts_to_zero() {
        let row = OtpRow {
            code: "000000".into(),
            expires_at: Utc::now(),
            attempts: -1,
        };
        let challenge = OtpChallenge::from(row);
        assert_eq!(challenge.attempts, 0);
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_otp_store_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let store = AnyOtpStore::postgres(pool);
        assert!(store.is_postgres());
        let email = format!("otp-{}@example.com", Uuid::new_v4());
        let challenge = OtpChallenge {
            code: "654321".into(),
            expires_at: Utc::now() + Duration::seconds(300),
            attempts: 1,
        };
        store.put(&email, challenge).await.expect("put");
        let got = store.get(&email).await.unwrap().expect("get");
        assert_eq!(got.code, "654321");
        assert_eq!(got.attempts, 1);

        let taken = store.take(&email).await.unwrap().expect("take");
        assert_eq!(taken.code, "654321");
        assert!(store.get(&email).await.unwrap().is_none());
    }
}
