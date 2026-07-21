//! Database layer placeholders (SQLx migrations land with auth/rooms).
//!
//! For P0 we only expose migration path constants and a pure SQL sanity helper
//! so the crate is testable without a live Postgres.

/// Expected migrations directory relative to backend workspace.
pub const MIGRATIONS_DIR: &str = "migrations";

/// Embedded list of known migration filenames (P1).
pub const MIGRATION_FILES: &[&str] = &["001_init.sql"];

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

/// Tables created by 001_init.sql (for smoke assertions without a live DB).
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_dir_constant() {
        assert_eq!(MIGRATIONS_DIR, "migrations");
        assert_eq!(MIGRATION_FILES, &["001_init.sql"]);
    }

    #[test]
    fn expected_tables_cover_p1() {
        let tables = expected_tables();
        assert!(tables.contains(&"users"));
        assert!(tables.contains(&"gift_orders"));
        assert!(tables.contains(&"admin_audit"));
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
}
