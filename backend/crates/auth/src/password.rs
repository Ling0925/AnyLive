//! Password hashing (argon2id) + policy helpers.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use anylive_common::{AppError, ErrorCode};
use password_hash::rand_core::OsRng;

/// Default minimum password length (override via `PASSWORD_MIN_LEN`).
pub const DEFAULT_PASSWORD_MIN_LEN: usize = 8;
/// Max accepted password length (DoS guard).
pub const PASSWORD_MAX_LEN: usize = 128;
/// Failed attempts before lockout.
pub const DEFAULT_LOGIN_MAX_ATTEMPTS: u32 = 5;
/// Lock duration after max attempts (seconds).
pub const DEFAULT_LOGIN_LOCK_SECS: i64 = 15 * 60;

/// Resolved password / lockout policy from env (or defaults).
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_len: usize,
    pub max_attempts: u32,
    pub lock_secs: i64,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_len: DEFAULT_PASSWORD_MIN_LEN,
            max_attempts: DEFAULT_LOGIN_MAX_ATTEMPTS,
            lock_secs: DEFAULT_LOGIN_LOCK_SECS,
        }
    }
}

impl PasswordPolicy {
    pub fn from_env() -> Self {
        let min_len = std::env::var("PASSWORD_MIN_LEN")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|n| *n >= 6 && *n <= 64)
            .unwrap_or(DEFAULT_PASSWORD_MIN_LEN);
        let max_attempts = std::env::var("LOGIN_MAX_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|n: &u32| *n >= 3 && *n <= 50)
            .unwrap_or(DEFAULT_LOGIN_MAX_ATTEMPTS);
        let lock_secs = std::env::var("LOGIN_LOCK_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|n: &i64| *n >= 30 && *n <= 86_400)
            .unwrap_or(DEFAULT_LOGIN_LOCK_SECS);
        Self {
            min_len,
            max_attempts,
            lock_secs,
        }
    }

    pub fn validate_password(&self, password: &str) -> Result<(), AppError> {
        let len = password.chars().count();
        if len < self.min_len {
            return Err(AppError::validation(format!(
                "password must be at least {} characters",
                self.min_len
            )));
        }
        if len > PASSWORD_MAX_LEN {
            return Err(AppError::validation(format!(
                "password must be at most {PASSWORD_MAX_LEN} characters"
            )));
        }
        Ok(())
    }
}

/// Hash a password with argon2id (PHC string).
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| {
            tracing::error!(error = %e, "argon2 hash failed");
            AppError::new(ErrorCode::Internal, "password hash failed")
        })
}

/// Verify password against a PHC hash string.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(password_hash).map_err(|e| {
        tracing::error!(error = %e, "invalid stored password hash");
        AppError::new(ErrorCode::Internal, "invalid password hash")
    })?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Generate a temporary password (readable alnum, length 12).
pub fn generate_temp_password() -> String {
    use rand_core::RngCore;
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut rng = OsRng;
    let mut out = String::with_capacity(12);
    for _ in 0..12 {
        let idx = (rng.next_u32() as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }
    out
}

/// Uniform invalid-credentials error (anti-enumeration).
pub fn invalid_credentials() -> AppError {
    AppError::new(
        ErrorCode::AuthInvalidCredentials,
        "invalid username/email or password",
    )
}

/// Account locked error.
pub fn account_locked() -> AppError {
    AppError::new(
        ErrorCode::AuthAccountLocked,
        "account temporarily locked; try again later",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let h = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }

    #[test]
    fn policy_rejects_short() {
        let p = PasswordPolicy::default();
        assert!(p.validate_password("short").is_err());
        assert!(p.validate_password("longenough").is_ok());
    }

    #[test]
    fn temp_password_length() {
        let t = generate_temp_password();
        assert_eq!(t.len(), 12);
    }
}
