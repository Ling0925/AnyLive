//! Email OTP generation and validation (dev-friendly).

use anylive_common::{AppError, ErrorCode};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::store::{OtpChallenge, OtpStore};
use crate::DEV_OTP_CODE;

/// Max failed verify attempts before the challenge is burned.
pub const OTP_MAX_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone)]
pub struct OtpConfig {
    /// OTP lifetime.
    pub ttl_secs: i64,
    /// When true, always accept [`DEV_OTP_CODE`] and store that code on send.
    /// Defaults to **false** (safe). Enable explicitly for local/dev only.
    pub dev_fixed_otp: bool,
}

impl Default for OtpConfig {
    fn default() -> Self {
        Self {
            // 5-minute OTP lifetime.
            ttl_secs: 5 * 60,
            // Secure default: never accept a fixed OTP unless explicitly enabled.
            dev_fixed_otp: false,
        }
    }
}

impl OtpConfig {
    /// Local/dev convenience: fixed OTP `123456`, still 5 min TTL.
    pub fn dev() -> Self {
        Self {
            ttl_secs: 5 * 60,
            dev_fixed_otp: true,
        }
    }
}

/// Normalized 4–8 digit OTP code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtpCode(String);

impl OtpCode {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let code = raw.trim();
        if code.len() < 4 || code.len() > 8 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::new(
                ErrorCode::AuthInvalidOtp,
                "OTP must be 4-8 digits",
            ));
        }
        Ok(Self(code.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Constant-time equality for equal-length digit strings (mitigates timing leaks).
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Cryptographically random 6-digit OTP (000000–999999), via UUID v4 CSPRNG.
fn generate_otp_code() -> String {
    let n = (Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{n:06}")
}

#[derive(Clone)]
pub struct OtpService<S: OtpStore> {
    store: S,
    config: OtpConfig,
}

impl<S: OtpStore> OtpService<S> {
    pub fn new(store: S, config: OtpConfig) -> Self {
        Self { store, config }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    /// Create/overwrite OTP for email. In dev mode the code is always `123456`.
    ///
    /// Returns the plaintext code for delivery (email/SMS). Callers must not log it.
    /// Enforces a per-email resend cooldown to slow enumeration/spam.
    pub async fn send(&self, email: &str) -> Result<String, AppError> {
        let email = normalize_email(email)?;
        // Resend throttle: reject if an unexpired challenge was issued too recently.
        if let Some(existing) = self.store.get(&email).await? {
            let issued_at = existing.expires_at - Duration::seconds(self.config.ttl_secs);
            let elapsed = Utc::now().signed_duration_since(issued_at).num_seconds();
            if elapsed >= 0 && elapsed < OTP_RESEND_COOLDOWN_SECS {
                return Err(AppError::new(
                    ErrorCode::RateLimited,
                    format!("wait {OTP_RESEND_COOLDOWN_SECS}s before requesting another OTP"),
                ));
            }
        }
        let code = if self.config.dev_fixed_otp {
            DEV_OTP_CODE.to_string()
        } else {
            generate_otp_code()
        };
        let expires_at = Utc::now() + Duration::seconds(self.config.ttl_secs);
        self.store
            .put(
                &email,
                OtpChallenge {
                    code: code.clone(),
                    expires_at,
                    attempts: 0,
                },
            )
            .await?;
        // Never log the OTP code itself.
        tracing::info!(%email, "otp issued (dev fixed={})", self.config.dev_fixed_otp);
        Ok(code)
    }

    /// Validate OTP for email; consumes the challenge on success.
    ///
    /// Uses take-first so concurrent successful verifies cannot both accept the
    /// same code (second caller sees "not found"). Failed attempts put the
    /// challenge back with an incremented counter under the store's write path.
    pub async fn verify(&self, email: &str, code: &str) -> Result<String, AppError> {
        let email = normalize_email(email)?;
        let submitted = OtpCode::parse(code)?;

        // Dev convenience: fixed OTP always works even without prior send.
        // NEVER enable outside local/dev — this is a total auth bypass.
        if self.config.dev_fixed_otp && constant_time_eq(submitted.as_str(), DEV_OTP_CODE) {
            let _ = self.store.take(&email).await;
            return Ok(email);
        }

        // Atomically claim the challenge so only one verifier can succeed.
        let mut challenge = self
            .store
            .take(&email)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::AuthInvalidOtp, "OTP not found or expired"))?;

        if challenge.expires_at < Utc::now() {
            // Already removed via take — do not put expired codes back.
            return Err(AppError::new(ErrorCode::AuthInvalidOtp, "OTP expired"));
        }

        if challenge.attempts >= OTP_MAX_ATTEMPTS {
            // Already removed via take.
            return Err(AppError::new(
                ErrorCode::AuthInvalidOtp,
                "too many OTP attempts",
            ));
        }

        if !constant_time_eq(&challenge.code, submitted.as_str()) {
            challenge.attempts += 1;
            // Burn after the failed attempt that hits the limit (leave deleted).
            if challenge.attempts < OTP_MAX_ATTEMPTS {
                self.store.put(&email, challenge).await?;
            }
            return Err(AppError::new(ErrorCode::AuthInvalidOtp, "invalid OTP"));
        }

        // Success: challenge already consumed by take.
        Ok(email)
    }
}

pub fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') || email.len() > 254 {
        return Err(AppError::validation("invalid email"));
    }
    let parts: Vec<_> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
        return Err(AppError::validation("invalid email"));
    }
    Ok(email)
}

/// Minimum seconds between OTP sends for the same email (resend throttle).
pub const OTP_RESEND_COOLDOWN_SECS: i64 = 30;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryOtpStore;

    fn svc(dev: bool) -> OtpService<InMemoryOtpStore> {
        OtpService::new(
            InMemoryOtpStore::default(),
            OtpConfig {
                ttl_secs: 300,
                dev_fixed_otp: dev,
            },
        )
    }

    #[tokio::test]
    async fn send_and_verify_dev_otp() {
        let s = svc(true);
        let code = s.send("User@Example.com").await.unwrap();
        assert_eq!(code, DEV_OTP_CODE);
        let email = s.verify("user@example.com", "123456").await.unwrap();
        assert_eq!(email, "user@example.com");
    }

    #[tokio::test]
    async fn reject_bad_code_format() {
        assert!(OtpCode::parse("12").is_err());
        assert!(OtpCode::parse("abcdef").is_err());
        assert!(OtpCode::parse("123456").is_ok());
    }

    #[tokio::test]
    async fn reject_wrong_otp_when_not_dev_fixed() {
        let s = svc(false);
        let code = s.send("a@b.co").await.unwrap();
        // CSPRNG may theoretically hit 000000; still verify wrong code fails.
        let wrong = if code == "000000" { "000001" } else { "000000" };
        let err = s.verify("a@b.co", wrong).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn prod_otp_roundtrip() {
        let s = svc(false);
        let code = s.send("prod@example.com").await.unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        let email = s.verify("prod@example.com", &code).await.unwrap();
        assert_eq!(email, "prod@example.com");
        // Consumed — second use fails.
        let err = s.verify("prod@example.com", &code).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn lockout_after_max_attempts() {
        let s = svc(false);
        let code = s.send("lock@example.com").await.unwrap();
        let wrong = if code == "000000" { "111111" } else { "000000" };
        for _ in 0..OTP_MAX_ATTEMPTS {
            let err = s.verify("lock@example.com", wrong).await.unwrap_err();
            assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
        }
        // Even the correct code is rejected once burned.
        let err = s.verify("lock@example.com", &code).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn expired_otp_rejected() {
        let s = OtpService::new(
            InMemoryOtpStore::default(),
            OtpConfig {
                ttl_secs: 0, // already expired on next tick
                dev_fixed_otp: false,
            },
        );
        let code = s.send("exp@example.com").await.unwrap();
        // Ensure wall clock advanced past expiry without requiring tokio "time" feature.
        std::thread::sleep(std::time::Duration::from_millis(5));
        let err = s.verify("exp@example.com", &code).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn dev_otp_works_without_send() {
        let s = svc(true);
        let email = s.verify("solo@example.com", DEV_OTP_CODE).await.unwrap();
        assert_eq!(email, "solo@example.com");
    }

    #[tokio::test]
    async fn concurrent_correct_verify_succeeds_only_once() {
        // take-first: two concurrent success verifies must not both accept the code.
        let s = svc(false);
        let code = s.send("race@example.com").await.unwrap();
        let (a, b) = tokio::join!(
            s.verify("race@example.com", &code),
            s.verify("race@example.com", &code),
        );
        let wins = [a.is_ok(), b.is_ok()].into_iter().filter(|x| *x).count();
        let loses = [a.is_err(), b.is_err()]
            .into_iter()
            .filter(|x| *x)
            .count();
        assert_eq!(wins, 1, "exactly one concurrent verify may succeed");
        assert_eq!(loses, 1, "the other must fail as consumed");
        // Challenge fully consumed — further verify fails.
        let err = s.verify("race@example.com", &code).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn default_config_disables_fixed_otp() {
        assert!(!OtpConfig::default().dev_fixed_otp);
        assert!(OtpConfig::dev().dev_fixed_otp);
    }

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq("123456", "123456"));
        assert!(!constant_time_eq("123456", "123457"));
        assert!(!constant_time_eq("1234", "12345"));
    }
}
