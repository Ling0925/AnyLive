//! Database layer placeholders (SQLx migrations land with auth/rooms).
//!
//! For P0 we only expose migration path constants and a pure SQL sanity helper
//! so the crate is testable without a live Postgres.

/// Expected migrations directory relative to backend workspace.
pub const MIGRATIONS_DIR: &str = "migrations";

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_dir_constant() {
        assert_eq!(MIGRATIONS_DIR, "migrations");
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
}
