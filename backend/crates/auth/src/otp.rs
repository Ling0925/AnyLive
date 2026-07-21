//! Email OTP generation and validation (dev-friendly).

use anylive_common::{AppError, ErrorCode};
use chrono::{Duration, Utc};

use crate::store::{OtpChallenge, OtpStore};
use crate::DEV_OTP_CODE;

#[derive(Debug, Clone)]
pub struct OtpConfig {
    /// OTP lifetime.
    pub ttl_secs: i64,
    /// When true, always accept [`DEV_OTP_CODE`] and store that code on send.
    pub dev_fixed_otp: bool,
}

impl Default for OtpConfig {
    fn default() -> Self {
        Self {
            // 5-minute OTP lifetime.
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
    pub async fn send(&self, email: &str) -> Result<String, AppError> {
        let email = normalize_email(email)?;
        let code = if self.config.dev_fixed_otp {
            DEV_OTP_CODE.to_string()
        } else {
            // Simple non-crypto OTP for future prod path; replace with CSPRNG.
            format!("{:06}", (Utc::now().timestamp_subsec_nanos() % 1_000_000))
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
        tracing::info!(%email, "otp issued (dev fixed={})", self.config.dev_fixed_otp);
        Ok(code)
    }

    /// Validate OTP for email; consumes the challenge on success.
    pub async fn verify(&self, email: &str, code: &str) -> Result<String, AppError> {
        let email = normalize_email(email)?;
        let submitted = OtpCode::parse(code)?;

        // Dev convenience: fixed OTP always works even without prior send.
        if self.config.dev_fixed_otp && submitted.as_str() == DEV_OTP_CODE {
            let _ = self.store.take(&email).await;
            return Ok(email);
        }

        let mut challenge = self
            .store
            .get(&email)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::AuthInvalidOtp, "OTP not found or expired"))?;

        if challenge.expires_at < Utc::now() {
            let _ = self.store.take(&email).await;
            return Err(AppError::new(ErrorCode::AuthInvalidOtp, "OTP expired"));
        }

        if challenge.attempts >= 5 {
            let _ = self.store.take(&email).await;
            return Err(AppError::new(
                ErrorCode::AuthInvalidOtp,
                "too many OTP attempts",
            ));
        }

        if challenge.code != submitted.as_str() {
            challenge.attempts += 1;
            self.store.put(&email, challenge).await?;
            return Err(AppError::new(ErrorCode::AuthInvalidOtp, "invalid OTP"));
        }

        let _ = self.store.take(&email).await;
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
        assert_ne!(code, "000000");
        let err = s.verify("a@b.co", "000000").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn dev_otp_works_without_send() {
        let s = svc(true);
        let email = s.verify("solo@example.com", DEV_OTP_CODE).await.unwrap();
        assert_eq!(email, "solo@example.com");
    }
}
