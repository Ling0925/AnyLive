//! Refresh-token dual store (`refresh_tokens` from `004_auth_sessions.sql`).
//!
//! Implements [`RefreshStore`] so `AuthService` can persist sessions when Postgres
//! is enabled. OTP remains in-memory for P1 (short TTL, process-local is acceptable).

use anylive_auth::{InMemoryRefreshStore, RefreshStore};
use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Postgres-backed [`RefreshStore`] (`refresh_tokens` table).
#[derive(Clone)]
pub struct PostgresRefreshStore {
    pool: PgPool,
}

impl PostgresRefreshStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres refresh store error");
    AppError::new(ErrorCode::Internal, "database error")
}

fn exp_to_dt(exp: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(exp, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap_or_else(Utc::now))
}

#[async_trait]
impl RefreshStore for PostgresRefreshStore {
    async fn insert(&self, jti: Uuid, user_id: UserId, exp: i64) -> Result<(), AppError> {
        let expires_at = exp_to_dt(exp);
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (jti, user_id, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (jti) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                expires_at = EXCLUDED.expires_at
            "#,
        )
        .bind(jti)
        .bind(user_id.0)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(())
    }

    async fn revoke(&self, jti: Uuid) -> Result<bool, AppError> {
        let res = sqlx::query(
            r#"
            DELETE FROM refresh_tokens
            WHERE jti = $1
            "#,
        )
        .bind(jti)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(res.rows_affected() > 0)
    }

    async fn is_active(&self, jti: Uuid) -> Result<bool, AppError> {
        let row = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT jti
            FROM refresh_tokens
            WHERE jti = $1
              AND expires_at >= now()
            "#,
        )
        .bind(jti)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.is_some())
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<usize, AppError> {
        let res = sqlx::query(
            r#"
            DELETE FROM refresh_tokens
            WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(res.rows_affected() as usize)
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyRefreshStore {
    Memory(InMemoryRefreshStore),
    Postgres(PostgresRefreshStore),
}

impl AnyRefreshStore {
    pub fn memory() -> Self {
        Self::Memory(InMemoryRefreshStore::default())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresRefreshStore::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }
}

#[async_trait]
impl RefreshStore for AnyRefreshStore {
    async fn insert(&self, jti: Uuid, user_id: UserId, exp: i64) -> Result<(), AppError> {
        match self {
            Self::Memory(s) => s.insert(jti, user_id, exp).await,
            Self::Postgres(s) => s.insert(jti, user_id, exp).await,
        }
    }

    async fn revoke(&self, jti: Uuid) -> Result<bool, AppError> {
        match self {
            Self::Memory(s) => s.revoke(jti).await,
            Self::Postgres(s) => s.revoke(jti).await,
        }
    }

    async fn is_active(&self, jti: Uuid) -> Result<bool, AppError> {
        match self {
            Self::Memory(s) => s.is_active(jti).await,
            Self::Postgres(s) => s.is_active(jti).await,
        }
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<usize, AppError> {
        match self {
            Self::Memory(s) => s.revoke_all_for_user(user_id).await,
            Self::Postgres(s) => s.revoke_all_for_user(user_id).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[tokio::test]
    async fn memory_backend_insert_revoke() {
        let store = AnyRefreshStore::memory();
        let jti = Uuid::new_v4();
        let uid = UserId::new();
        let exp = Utc::now().timestamp() + 3600;
        store.insert(jti, uid, exp).await.unwrap();
        assert!(store.is_active(jti).await.unwrap());
        assert!(!store.is_postgres());
        assert!(store.revoke(jti).await.unwrap());
        assert!(!store.is_active(jti).await.unwrap());
        assert!(!store.revoke(jti).await.unwrap());
    }

    #[tokio::test]
    async fn memory_backend_revoke_all_for_user() {
        let store = AnyRefreshStore::memory();
        let uid = UserId::new();
        let other = UserId::new();
        let exp = Utc::now().timestamp() + 3600;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        store.insert(a, uid, exp).await.unwrap();
        store.insert(b, uid, exp).await.unwrap();
        store.insert(c, other, exp).await.unwrap();
        let n = store.revoke_all_for_user(uid).await.unwrap();
        assert_eq!(n, 2);
        assert!(!store.is_active(a).await.unwrap());
        assert!(!store.is_active(b).await.unwrap());
        assert!(store.is_active(c).await.unwrap());
    }

    #[tokio::test]
    async fn memory_expired_is_not_active() {
        let store = AnyRefreshStore::memory();
        let jti = Uuid::new_v4();
        store
            .insert(jti, UserId::new(), Utc::now().timestamp() - 10)
            .await
            .unwrap();
        assert!(!store.is_active(jti).await.unwrap());
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_refresh_store_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let users = crate::PostgresUserStore::new(pool.clone());
        let user = users
            .upsert_by_email(&format!("refresh-{}@example.com", Uuid::new_v4()))
            .await
            .expect("user");

        let store = AnyRefreshStore::postgres(pool);
        assert!(store.is_postgres());
        let jti = Uuid::new_v4();
        let exp = Utc::now().timestamp() + 3600;
        store.insert(jti, user.id, exp).await.expect("insert");
        assert!(store.is_active(jti).await.unwrap());
        assert!(store.revoke(jti).await.unwrap());
        assert!(!store.is_active(jti).await.unwrap());

        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        store.insert(a, user.id, exp).await.unwrap();
        store.insert(b, user.id, exp).await.unwrap();
        let n = store.revoke_all_for_user(user.id).await.unwrap();
        assert_eq!(n, 2);
        assert!(!store.is_active(a).await.unwrap());
    }
}
