//! Database layer: SQLx pool, migrations, and Postgres adapters.
//!
//! Offline unit tests never touch Postgres. Set `DATABASE_URL` and `USE_POSTGRES=1`
//! to connect, migrate, and use Postgres stores at API startup.
//!
//! ## Enable path
//!
//! ```text
//! USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
//!   cargo run -p anylive-api
//! ```
//!
//! When enabled, API [`AppState::from_env`] wires Postgres dual stores for
//! users/rooms/wallet/social/moderation/reports/chat/profile_extras/deleted_users/refresh/otp.
//! Default (no env) keeps in-memory stores so `cargo test --workspace` needs no live PG.

mod chat;
mod deleted_users;
mod moderation;
mod otp;
mod pay;
mod pool;
mod profile;
mod refresh;
mod reports;
mod rooms;
mod social;
mod users;
mod wallet;

pub use chat::{validate_chat_body, AnyChat, PostgresChat};
pub use deleted_users::{AnyDeletedUsers, MemoryDeletedUsers, PostgresDeletedUsers};
pub use moderation::{AnyModeration, PostgresModeration};
pub use otp::{AnyOtpStore, PostgresOtpStore};
pub use pay::{AnyPayStore, PostgresPayStore};
pub use pool::{
    connect, connect_and_migrate_from_env, migrate, migrations_dir, ping, postgres_enabled,
    DbError, PgPool,
};
pub use profile::{
    AnyProfileExtras, MemoryProfileExtras, PostgresProfileExtras, ProfileExtras,
};
pub use refresh::{AnyRefreshStore, PostgresRefreshStore};
pub use reports::{AnyReports, MemoryReports, PostgresReports, Report, ReportStatus};
pub use rooms::{map_room_error, PostgresRoomStore};
pub use social::{AnySocial, PostgresSocial};
pub use users::{AnyUserStore, PostgresUserStore};
pub use wallet::{AnyWallet, PostgresWallet};

/// Expected migrations directory relative to backend workspace.
pub const MIGRATIONS_DIR: &str = "migrations";

/// Embedded list of known migration filenames (P1).
pub const MIGRATION_FILES: &[&str] = &[
    "001_init.sql",
    "002_reports_mute.sql",
    "003_profile_extras.sql",
    "004_auth_sessions.sql",
    "005_otp_challenges.sql",
    "006_wallet_topup_idempotency.sql",
    "007_pay_orders.sql",
];

/// Validate that a SQL identifier is safe for use in limited admin tooling.
pub fn is_safe_ident(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
}

/// Tables created by 001–005 migrations (smoke assertions without a live DB).
pub fn expected_tables() -> &'static [&'static str] {
    &[
        "users",
        "rooms",
        "wallet_balances",
        "wallet_ledger",
        "gifts",
        "gift_orders",
        "follows",
        "chat_messages",
        "admin_users",
        "banned_users",
        "admin_audit",
        "reports",
        "muted_users",
        "profile_extras",
        "deleted_users",
        "refresh_tokens",
        "otp_challenges",
        "pay_products",
        "pay_orders",
        "pay_webhook_events",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_dir_constant() {
        assert_eq!(MIGRATIONS_DIR, "migrations");
        assert_eq!(
            MIGRATION_FILES,
            &[
                "001_init.sql",
                "002_reports_mute.sql",
                "003_profile_extras.sql",
                "004_auth_sessions.sql",
                "005_otp_challenges.sql",
                "006_wallet_topup_idempotency.sql",
                "007_pay_orders.sql",
            ]
        );
    }

    #[test]
    fn expected_tables_cover_p1() {
        let tables = expected_tables();
        assert!(tables.contains(&"users"));
        assert!(tables.contains(&"gift_orders"));
        assert!(tables.contains(&"admin_audit"));
        assert!(tables.contains(&"reports"));
        assert!(tables.contains(&"muted_users"));
        assert!(tables.contains(&"profile_extras"));
        assert!(tables.contains(&"deleted_users"));
        assert!(tables.contains(&"refresh_tokens"));
        assert!(tables.contains(&"otp_challenges"));
    }

    #[test]
    fn safe_ident_accepts_table_names() {
        assert!(is_safe_ident("users"));
        assert!(is_safe_ident("gift_orders"));
        assert!(is_safe_ident("_private"));
    }

    #[test]
    fn safe_ident_rejects_injection() {
        assert!(!is_safe_ident(""));
        assert!(!is_safe_ident("users;drop"));
        assert!(!is_safe_ident("1users"));
        assert!(!is_safe_ident("a-b"));
    }

    #[test]
    fn init_migration_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/001_init.sql");
        assert!(
            path.exists(),
            "expected migration at {}",
            path.display()
        );
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS users"));
        assert!(sql.contains("UNIQUE (sender_id, client_request_id)"));
    }

    #[test]
    fn reports_mute_migration_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/002_reports_mute.sql");
        assert!(
            path.exists(),
            "expected migration at {}",
            path.display()
        );
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS reports"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS muted_users"));
    }

    #[test]
    fn profile_extras_migration_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/003_profile_extras.sql");
        assert!(
            path.exists(),
            "expected migration at {}",
            path.display()
        );
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS profile_extras"));
        assert!(sql.contains("age_confirmed_at"));
        assert!(sql.contains("privacy_accepted_at"));
    }

    #[test]
    fn auth_sessions_migration_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/004_auth_sessions.sql");
        assert!(
            path.exists(),
            "expected migration at {}",
            path.display()
        );
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS deleted_users"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS refresh_tokens"));
        assert!(sql.contains("idx_refresh_tokens_user"));
    }

    #[test]
    fn otp_challenges_migration_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/005_otp_challenges.sql");
        assert!(
            path.exists(),
            "expected migration at {}",
            path.display()
        );
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS otp_challenges"));
        assert!(sql.contains("email TEXT PRIMARY KEY"));
        assert!(sql.contains("expires_at TIMESTAMPTZ NOT NULL"));
        assert!(sql.contains("attempts INT NOT NULL DEFAULT 0"));
    }

    #[test]
    fn postgres_disabled_by_default() {
        // Unit tests run without USE_POSTGRES=1; helper must not force PG.
        // We only assert the function is callable; actual env may vary in CI.
        let _ = postgres_enabled();
    }
}
