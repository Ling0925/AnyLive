//! Soft-deleted accounts dual store (`deleted_users` from `004_auth_sessions.sql`).
//!
//! Used by compliance delete/export and auth guards so deleted accounts stay blocked
//! across process restarts when Postgres is enabled.

use std::collections::HashSet;
use std::sync::Arc;

use anylive_domain::UserId;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Process-local set of soft-deleted user IDs.
#[derive(Clone, Default)]
pub struct MemoryDeletedUsers {
    inner: Arc<Mutex<HashSet<UserId>>>,
}

impl MemoryDeletedUsers {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn mark_deleted(&self, user_id: UserId) {
        self.inner.lock().await.insert(user_id);
    }

    pub async fn is_deleted(&self, user_id: UserId) -> bool {
        self.inner.lock().await.contains(&user_id)
    }
}

/// Postgres-backed soft-delete set (`deleted_users` table).
#[derive(Clone)]
pub struct PostgresDeletedUsers {
    pool: PgPool,
}

impl PostgresDeletedUsers {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn mark_deleted(&self, user_id: UserId) {
        let res = sqlx::query(
            r#"
            INSERT INTO deleted_users (user_id)
            VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id.0)
        .execute(&self.pool)
        .await;
        if let Err(err) = res {
            tracing::error!(error = %err, user_id = %user_id.0, "postgres mark_deleted failed");
        }
    }

    pub async fn is_deleted(&self, user_id: UserId) -> bool {
        let res = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT user_id
            FROM deleted_users
            WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await;
        match res {
            Ok(row) => row.is_some(),
            Err(err) => {
                // Fail closed: if we cannot check, treat as deleted so soft-delete
                // cannot be bypassed by a transient DB outage.
                tracing::error!(error = %err, user_id = %user_id.0, "postgres is_deleted failed");
                true
            }
        }
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyDeletedUsers {
    Memory(MemoryDeletedUsers),
    Postgres(PostgresDeletedUsers),
}

impl AnyDeletedUsers {
    pub fn memory() -> Self {
        Self::Memory(MemoryDeletedUsers::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresDeletedUsers::new(pool))
    }

    /// Alias for [`Self::memory`] (matches previous `DeletedUsers::new()` call sites).
    pub fn new() -> Self {
        Self::memory()
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn mark_deleted(&self, user_id: UserId) {
        match self {
            Self::Memory(s) => s.mark_deleted(user_id).await,
            Self::Postgres(s) => s.mark_deleted(user_id).await,
        }
    }

    pub async fn is_deleted(&self, user_id: UserId) -> bool {
        match self {
            Self::Memory(s) => s.is_deleted(user_id).await,
            Self::Postgres(s) => s.is_deleted(user_id).await,
        }
    }
}

impl Default for AnyDeletedUsers {
    fn default() -> Self {
        Self::memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[tokio::test]
    async fn memory_mark_and_check() {
        let set = AnyDeletedUsers::memory();
        let id = UserId::new();
        assert!(!set.is_deleted(id).await);
        assert!(!set.is_postgres());
        set.mark_deleted(id).await;
        assert!(set.is_deleted(id).await);
        // Idempotent.
        set.mark_deleted(id).await;
        assert!(set.is_deleted(id).await);
    }

    #[tokio::test]
    async fn new_alias_is_memory() {
        let set = AnyDeletedUsers::new();
        assert!(!set.is_postgres());
        let id = UserId::new();
        set.mark_deleted(id).await;
        assert!(set.is_deleted(id).await);
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_deleted_users_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let users = crate::PostgresUserStore::new(pool.clone());
        let user = users
            .upsert_by_email(&format!("deleted-{}@example.com", Uuid::new_v4()))
            .await
            .expect("user");

        let set = AnyDeletedUsers::postgres(pool);
        assert!(set.is_postgres());
        assert!(!set.is_deleted(user.id).await);
        set.mark_deleted(user.id).await;
        assert!(set.is_deleted(user.id).await);
        // Idempotent upsert.
        set.mark_deleted(user.id).await;
        assert!(set.is_deleted(user.id).await);
    }
}
