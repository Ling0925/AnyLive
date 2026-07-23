//! [`PostgresSocial`] + dual [`AnySocial`] matching [`MemorySocial`] surface.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use anylive_social::MemorySocial;
use sqlx::PgPool;
use uuid::Uuid;

/// Postgres-backed social graph (`follows` table from `001_init.sql`).
#[derive(Clone)]
pub struct PostgresSocial {
    pool: PgPool,
}

impl PostgresSocial {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn follow(&self, follower: UserId, followee: UserId) -> Result<(), AppError> {
        if follower.0 == followee.0 {
            return Err(AppError::validation("cannot follow yourself"));
        }
        sqlx::query(
            r#"
            INSERT INTO follows (follower_id, followee_id)
            VALUES ($1, $2)
            ON CONFLICT (follower_id, followee_id) DO NOTHING
            "#,
        )
        .bind(follower.0)
        .bind(followee.0)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(())
    }

    pub async fn unfollow(&self, follower: UserId, followee: UserId) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM follows
            WHERE follower_id = $1 AND followee_id = $2
            "#,
        )
        .bind(follower.0)
        .bind(followee.0)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(())
    }

    pub async fn is_following(&self, follower: UserId, followee: UserId) -> bool {
        let row: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT true
            FROM follows
            WHERE follower_id = $1 AND followee_id = $2
            "#,
        )
        .bind(follower.0)
        .bind(followee.0)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();
        row.unwrap_or(false)
    }

    pub async fn following_ids(&self, follower: UserId) -> Vec<UserId> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT followee_id
            FROM follows
            WHERE follower_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(follower.0)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "postgres following_ids failed");
            Vec::new()
        });
        rows.into_iter().map(UserId).collect()
    }

    pub async fn follower_count(&self, user: UserId) -> u64 {
        let n: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM follows
            WHERE followee_id = $1
            "#,
        )
        .bind(user.0)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        n.max(0) as u64
    }

    pub async fn following_count(&self, user: UserId) -> u64 {
        let n: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM follows
            WHERE follower_id = $1
            "#,
        )
        .bind(user.0)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        n.max(0) as u64
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnySocial {
    Memory(MemorySocial),
    Postgres(PostgresSocial),
}

impl AnySocial {
    pub fn memory() -> Self {
        Self::Memory(MemorySocial::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresSocial::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn follow(&self, follower: UserId, followee: UserId) -> Result<(), AppError> {
        match self {
            Self::Memory(s) => s.follow(follower, followee).await,
            Self::Postgres(s) => s.follow(follower, followee).await,
        }
    }

    pub async fn unfollow(&self, follower: UserId, followee: UserId) -> Result<(), AppError> {
        match self {
            Self::Memory(s) => s.unfollow(follower, followee).await,
            Self::Postgres(s) => s.unfollow(follower, followee).await,
        }
    }

    pub async fn is_following(&self, follower: UserId, followee: UserId) -> bool {
        match self {
            Self::Memory(s) => s.is_following(follower, followee).await,
            Self::Postgres(s) => s.is_following(follower, followee).await,
        }
    }

    pub async fn following_ids(&self, follower: UserId) -> Vec<UserId> {
        match self {
            Self::Memory(s) => s.following_ids(follower).await,
            Self::Postgres(s) => s.following_ids(follower).await,
        }
    }

    pub async fn follower_count(&self, user: UserId) -> u64 {
        match self {
            Self::Memory(s) => s.follower_count(user).await,
            Self::Postgres(s) => s.follower_count(user).await,
        }
    }

    pub async fn following_count(&self, user: UserId) -> u64 {
        match self {
            Self::Memory(s) => s.following_count(user).await,
            Self::Postgres(s) => s.following_count(user).await,
        }
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres social store error");
    if let sqlx::Error::Database(db) = &err {
        if db.constraint().is_some_and(|c| c.ends_with("_fkey")) {
            return AppError::validation("follower or followee user does not exist");
        }
        if db.constraint() == Some("follows_check") {
            return AppError::validation("cannot follow yourself");
        }
    }
    AppError::new(ErrorCode::Internal, "database error")
}

/// Pure SQL fragments (offline-testable, no live DB).
#[allow(dead_code)]
pub mod sql {
    pub const INSERT_FOLLOW: &str = r#"
            INSERT INTO follows (follower_id, followee_id)
            VALUES ($1, $2)
            ON CONFLICT (follower_id, followee_id) DO NOTHING
            "#;

    pub const DELETE_FOLLOW: &str = r#"
            DELETE FROM follows
            WHERE follower_id = $1 AND followee_id = $2
            "#;

    pub const SELECT_IS_FOLLOWING: &str = r#"
            SELECT true
            FROM follows
            WHERE follower_id = $1 AND followee_id = $2
            "#;

    pub const SELECT_FOLLOWING_IDS: &str = r#"
            SELECT followee_id
            FROM follows
            WHERE follower_id = $1
            ORDER BY created_at DESC
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[test]
    fn sql_fragments_mention_follows() {
        assert!(sql::INSERT_FOLLOW.contains("INSERT INTO follows"));
        assert!(sql::DELETE_FOLLOW.contains("DELETE FROM follows"));
        assert!(sql::SELECT_IS_FOLLOWING.contains("follower_id = $1"));
        assert!(sql::SELECT_FOLLOWING_IDS.contains("ORDER BY created_at DESC"));
    }

    #[tokio::test]
    async fn memory_backend_follow_unfollow() {
        let s = AnySocial::memory();
        let a = UserId::new();
        let b = UserId::new();
        s.follow(a, b).await.unwrap();
        assert!(s.is_following(a, b).await);
        assert_eq!(s.following_ids(a).await.len(), 1);
        s.unfollow(a, b).await.unwrap();
        assert!(!s.is_following(a, b).await);
        assert!(!s.is_postgres());
    }

    #[tokio::test]
    async fn memory_backend_reject_self_follow() {
        let s = AnySocial::memory();
        let a = UserId::new();
        assert!(s.follow(a, a).await.is_err());
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_social_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let users = crate::PostgresUserStore::new(pool.clone());
        let a = users
            .upsert_by_email(&format!("social-a-{}@example.com", Uuid::new_v4()))
            .await
            .expect("a");
        let b = users
            .upsert_by_email(&format!("social-b-{}@example.com", Uuid::new_v4()))
            .await
            .expect("b");

        let s = PostgresSocial::new(pool);
        s.follow(a.id, b.id).await.unwrap();
        s.follow(a.id, b.id).await.unwrap(); // idempotent
        assert!(s.is_following(a.id, b.id).await);
        assert_eq!(s.following_ids(a.id).await, vec![b.id]);
        s.unfollow(a.id, b.id).await.unwrap();
        assert!(!s.is_following(a.id, b.id).await);
        assert!(s.follow(a.id, a.id).await.is_err());
    }
}
