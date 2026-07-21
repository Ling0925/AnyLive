//! Profile extras dual store: age confirmation + privacy acceptance.
//!
//! Kept off the `users` table (`profile_extras` from `003_profile_extras.sql`).
//! Display name lives on UserStore; these flags live here.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Per-user age / privacy declaration timestamps.
#[derive(Debug, Clone, Default)]
pub struct ProfileExtras {
    pub age_confirmed_at: Option<DateTime<Utc>>,
    pub privacy_accepted_at: Option<DateTime<Utc>>,
}

impl ProfileExtras {
    pub fn age_confirmed(&self) -> bool {
        self.age_confirmed_at.is_some()
    }

    pub fn privacy_accepted(&self) -> bool {
        self.privacy_accepted_at.is_some()
    }
}

/// Process-local store for age/privacy flags.
#[derive(Clone, Default)]
pub struct MemoryProfileExtras {
    inner: Arc<RwLock<HashMap<Uuid, ProfileExtras>>>,
}

impl MemoryProfileExtras {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, user_id: UserId) -> ProfileExtras {
        self.inner
            .read()
            .await
            .get(&user_id.0)
            .cloned()
            .unwrap_or_default()
    }

    /// Set age confirmation. `true` stamps now; `false` clears.
    pub async fn set_age_confirmed(&self, user_id: UserId, confirmed: bool) -> ProfileExtras {
        let mut g = self.inner.write().await;
        let entry = g.entry(user_id.0).or_default();
        entry.age_confirmed_at = if confirmed {
            Some(Utc::now())
        } else {
            None
        };
        entry.clone()
    }

    /// Set privacy acceptance. `true` stamps now; `false` clears.
    pub async fn set_privacy_accepted(&self, user_id: UserId, accepted: bool) -> ProfileExtras {
        let mut g = self.inner.write().await;
        let entry = g.entry(user_id.0).or_default();
        entry.privacy_accepted_at = if accepted {
            Some(Utc::now())
        } else {
            None
        };
        entry.clone()
    }

    /// Apply optional age/privacy patches and return the resulting extras.
    pub async fn patch(
        &self,
        user_id: UserId,
        age_confirmed: Option<bool>,
        privacy_accepted: Option<bool>,
    ) -> ProfileExtras {
        let mut g = self.inner.write().await;
        let entry = g.entry(user_id.0).or_default();
        if let Some(v) = age_confirmed {
            entry.age_confirmed_at = if v { Some(Utc::now()) } else { None };
        }
        if let Some(v) = privacy_accepted {
            entry.privacy_accepted_at = if v { Some(Utc::now()) } else { None };
        }
        entry.clone()
    }
}

/// Postgres-backed profile extras (`profile_extras` table from `003_profile_extras.sql`).
#[derive(Clone)]
pub struct PostgresProfileExtras {
    pool: PgPool,
}

impl PostgresProfileExtras {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn get(&self, user_id: UserId) -> ProfileExtras {
        let row = sqlx::query_as::<_, ProfileExtrasRow>(sql::SELECT_EXTRAS)
            .bind(user_id.0)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(error = %err, "postgres profile_extras get failed");
                None
            });
        row.map(ProfileExtras::from).unwrap_or_default()
    }

    pub async fn set_age_confirmed(&self, user_id: UserId, confirmed: bool) -> ProfileExtras {
        self.patch(user_id, Some(confirmed), None).await
    }

    pub async fn set_privacy_accepted(&self, user_id: UserId, accepted: bool) -> ProfileExtras {
        self.patch(user_id, None, Some(accepted)).await
    }

    pub async fn patch(
        &self,
        user_id: UserId,
        age_confirmed: Option<bool>,
        privacy_accepted: Option<bool>,
    ) -> ProfileExtras {
        // Load current so partial patches preserve the other column.
        let mut current = self.get(user_id).await;
        if let Some(v) = age_confirmed {
            current.age_confirmed_at = if v { Some(Utc::now()) } else { None };
        }
        if let Some(v) = privacy_accepted {
            current.privacy_accepted_at = if v { Some(Utc::now()) } else { None };
        }

        let row = sqlx::query_as::<_, ProfileExtrasRow>(sql::UPSERT_EXTRAS)
            .bind(user_id.0)
            .bind(current.age_confirmed_at)
            .bind(current.privacy_accepted_at)
            .fetch_one(&self.pool)
            .await;

        match row {
            Ok(r) => ProfileExtras::from(r),
            Err(err) => {
                tracing::error!(error = %err, "postgres profile_extras patch failed");
                // Fall back to the computed value so callers still see the patch intent offline
                // of a transient write failure; production should monitor the error log.
                current
            }
        }
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyProfileExtras {
    Memory(MemoryProfileExtras),
    Postgres(PostgresProfileExtras),
}

impl AnyProfileExtras {
    pub fn memory() -> Self {
        Self::Memory(MemoryProfileExtras::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresProfileExtras::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn get(&self, user_id: UserId) -> ProfileExtras {
        match self {
            Self::Memory(s) => s.get(user_id).await,
            Self::Postgres(s) => s.get(user_id).await,
        }
    }

    pub async fn set_age_confirmed(&self, user_id: UserId, confirmed: bool) -> ProfileExtras {
        match self {
            Self::Memory(s) => s.set_age_confirmed(user_id, confirmed).await,
            Self::Postgres(s) => s.set_age_confirmed(user_id, confirmed).await,
        }
    }

    pub async fn set_privacy_accepted(&self, user_id: UserId, accepted: bool) -> ProfileExtras {
        match self {
            Self::Memory(s) => s.set_privacy_accepted(user_id, accepted).await,
            Self::Postgres(s) => s.set_privacy_accepted(user_id, accepted).await,
        }
    }

    pub async fn patch(
        &self,
        user_id: UserId,
        age_confirmed: Option<bool>,
        privacy_accepted: Option<bool>,
    ) -> ProfileExtras {
        match self {
            Self::Memory(s) => s.patch(user_id, age_confirmed, privacy_accepted).await,
            Self::Postgres(s) => s.patch(user_id, age_confirmed, privacy_accepted).await,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ProfileExtrasRow {
    #[allow(dead_code)]
    user_id: Uuid,
    age_confirmed_at: Option<DateTime<Utc>>,
    privacy_accepted_at: Option<DateTime<Utc>>,
}

impl From<ProfileExtrasRow> for ProfileExtras {
    fn from(r: ProfileExtrasRow) -> Self {
        Self {
            age_confirmed_at: r.age_confirmed_at,
            privacy_accepted_at: r.privacy_accepted_at,
        }
    }
}

#[allow(dead_code)]
fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres profile_extras store error");
    if let sqlx::Error::Database(db) = &err {
        if db.constraint().is_some_and(|c| c.ends_with("_fkey")) {
            return AppError::validation("user does not exist");
        }
    }
    AppError::new(ErrorCode::Internal, "database error")
}

/// Pure SQL fragments (offline-testable, no live DB).
#[allow(dead_code)]
pub mod sql {
    pub const SELECT_EXTRAS: &str = r#"
            SELECT user_id, age_confirmed_at, privacy_accepted_at
            FROM profile_extras
            WHERE user_id = $1
            "#;

    pub const UPSERT_EXTRAS: &str = r#"
            INSERT INTO profile_extras (user_id, age_confirmed_at, privacy_accepted_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET
                age_confirmed_at = EXCLUDED.age_confirmed_at,
                privacy_accepted_at = EXCLUDED.privacy_accepted_at
            RETURNING user_id, age_confirmed_at, privacy_accepted_at
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[test]
    fn sql_fragments_mention_profile_extras() {
        assert!(sql::SELECT_EXTRAS.contains("FROM profile_extras"));
        assert!(sql::UPSERT_EXTRAS.contains("INSERT INTO profile_extras"));
        assert!(sql::UPSERT_EXTRAS.contains("ON CONFLICT (user_id)"));
    }

    #[tokio::test]
    async fn memory_defaults_false() {
        let s = AnyProfileExtras::memory();
        let e = s.get(UserId::new()).await;
        assert!(!e.age_confirmed());
        assert!(!e.privacy_accepted());
        assert!(!s.is_postgres());
    }

    #[tokio::test]
    async fn memory_patch_sets_and_clears() {
        let s = AnyProfileExtras::memory();
        let id = UserId::new();
        let e = s.patch(id, Some(true), Some(true)).await;
        assert!(e.age_confirmed());
        assert!(e.privacy_accepted());
        assert!(e.age_confirmed_at.is_some());
        assert!(e.privacy_accepted_at.is_some());

        let e = s.patch(id, Some(false), None).await;
        assert!(!e.age_confirmed());
        assert!(e.privacy_accepted());
    }

    #[tokio::test]
    async fn memory_setters() {
        let s = AnyProfileExtras::memory();
        let id = UserId::new();
        let e = s.set_age_confirmed(id, true).await;
        assert!(e.age_confirmed());
        assert!(!e.privacy_accepted());
        let e = s.set_privacy_accepted(id, true).await;
        assert!(e.age_confirmed());
        assert!(e.privacy_accepted());
        let e = s.set_age_confirmed(id, false).await;
        assert!(!e.age_confirmed());
        assert!(e.privacy_accepted());
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_profile_extras_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let users = crate::PostgresUserStore::new(pool.clone());
        let user = users
            .upsert_by_email(&format!("profile-extras-{}@example.com", Uuid::new_v4()))
            .await
            .expect("user");

        let s = AnyProfileExtras::postgres(pool);
        assert!(s.is_postgres());
        let e = s.get(user.id).await;
        assert!(!e.age_confirmed());
        assert!(!e.privacy_accepted());

        let e = s.patch(user.id, Some(true), Some(true)).await;
        assert!(e.age_confirmed());
        assert!(e.privacy_accepted());

        let e = s.get(user.id).await;
        assert!(e.age_confirmed());
        assert!(e.privacy_accepted());

        let e = s.patch(user.id, Some(false), None).await;
        assert!(!e.age_confirmed());
        assert!(e.privacy_accepted());
    }
}
