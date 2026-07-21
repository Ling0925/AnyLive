//! PgPool connect + migration apply.

use std::path::PathBuf;
use std::time::Duration;

use anylive_common::{AppError, ErrorCode};
use sqlx::postgres::PgPoolOptions;
pub use sqlx::PgPool;

/// Errors from connect / migrate (mapped to [`AppError`] at API boundaries).
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("DATABASE_URL is not set")]
    MissingDatabaseUrl,
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<DbError> for AppError {
    fn from(value: DbError) -> Self {
        AppError::new(ErrorCode::Internal, value.to_string())
    }
}

/// True when both `USE_POSTGRES=1` and `DATABASE_URL` are present.
///
/// Default is false so `cargo test --workspace` stays on in-memory stores.
pub fn postgres_enabled() -> bool {
    matches!(std::env::var("USE_POSTGRES").as_deref(), Ok("1") | Ok("true"))
        && std::env::var("DATABASE_URL").is_ok()
}

/// Connect a Postgres pool from `DATABASE_URL` (or an explicit URL).
pub async fn connect(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Apply embedded SQL migrations (compiled into the binary via `sqlx::migrate!`).
///
/// Embedding avoids depending on a source-tree `migrations/` path at runtime,
/// so packaged binaries still migrate correctly. Offline unit tests do not
/// execute this path unless `USE_POSTGRES=1` is set.
pub async fn migrate(pool: &PgPool) -> Result<(), DbError> {
    // Path is relative to this crate's Cargo.toml (`crates/db` → `backend/migrations`).
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}

/// Absolute path to `backend/migrations` (for tests / tooling that inspect SQL on disk).
pub fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

/// Connect using `DATABASE_URL` env and run migrations.
pub async fn connect_and_migrate_from_env() -> Result<PgPool, DbError> {
    let url = std::env::var("DATABASE_URL").map_err(|_| DbError::MissingDatabaseUrl)?;
    let pool = connect(&url).await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Cheap liveness check used by readiness probes when a pool is configured.
pub async fn ping(pool: &PgPool) -> Result<(), DbError> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_dir_points_at_init_sql() {
        let dir = migrations_dir();
        assert!(
            dir.join("001_init.sql").exists(),
            "missing {}",
            dir.display()
        );
        assert!(
            dir.join("002_reports_mute.sql").exists(),
            "missing 002 under {}",
            dir.display()
        );
    }
}
