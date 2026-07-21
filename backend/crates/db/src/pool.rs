//! PgPool connect + migration apply.

use std::path::PathBuf;

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
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Apply SQL migrations from the workspace `migrations/` directory.
///
/// Path is resolved relative to this crate's `CARGO_MANIFEST_DIR` so the binary
/// works regardless of process cwd (as long as the source tree is present).
/// For production packaging, prefer embedding via `sqlx::migrate!` once the
/// layout is frozen; runtime path keeps offline unit tests free of compile-time
/// DB requirements.
pub async fn migrate(pool: &PgPool) -> Result<(), DbError> {
    let dir = migrations_dir();
    let migrator = sqlx::migrate::Migrator::new(dir.as_path()).await?;
    migrator.run(pool).await?;
    Ok(())
}

/// Absolute path to `backend/migrations`.
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
    }
}
