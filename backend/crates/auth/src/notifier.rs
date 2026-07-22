//! Delivery ports for OTP / notifications.
//!
//! Production must wire a real [`OtpNotifier`] (log, SMTP, SMS, etc.).
//! Local/dev may use [`LogOtpNotifier`] or [`NoopOtpNotifier`] only when
//! fixed OTP is explicitly enabled.

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use async_trait::async_trait;

/// Port for delivering a one-time password to an email address.
#[async_trait]
pub trait OtpNotifier: Send + Sync {
    /// Deliver `code` to `email`. Implementations must not log the code.
    async fn send_otp(&self, email: &str, code: &str) -> Result<(), AppError>;
}

/// Shared notifier handle used by [`crate::AuthService`].
pub type SharedOtpNotifier = Arc<dyn OtpNotifier>;

/// Logs only that an OTP was issued (never the code). Suitable for local
/// inspection when a real mailer is not configured.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogOtpNotifier;

#[async_trait]
impl OtpNotifier for LogOtpNotifier {
    async fn send_otp(&self, email: &str, code: &str) -> Result<(), AppError> {
        // Intentionally omit the code from logs.
        let _ = code;
        tracing::info!(%email, "otp delivery: LogOtpNotifier (code not logged)");
        Ok(())
    }
}

/// No-op delivery. Only safe when fixed/dev OTP is enabled (caller never needs
/// the real code). Production startup must reject this combination.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopOtpNotifier;

#[async_trait]
impl OtpNotifier for NoopOtpNotifier {
    async fn send_otp(&self, email: &str, code: &str) -> Result<(), AppError> {
        let _ = (email, code);
        Ok(())
    }
}

/// Fails every delivery — used to force configuration of a real mailer.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnconfiguredOtpNotifier;

#[async_trait]
impl OtpNotifier for UnconfiguredOtpNotifier {
    async fn send_otp(&self, email: &str, _code: &str) -> Result<(), AppError> {
        tracing::error!(%email, "otp delivery unconfigured");
        Err(AppError::new(
            ErrorCode::Internal,
            "OTP delivery is not configured",
        ))
    }
}

/// Build notifier from env:
/// - `OTP_NOTIFIER=log` → [`LogOtpNotifier`]
/// - `OTP_NOTIFIER=noop` → [`NoopOtpNotifier`]
/// - unset / other → [`UnconfiguredOtpNotifier`] (fail closed on send)
pub fn otp_notifier_from_env() -> SharedOtpNotifier {
    match std::env::var("OTP_NOTIFIER")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "log" => Arc::new(LogOtpNotifier),
        "noop" => Arc::new(NoopOtpNotifier),
        _ => Arc::new(UnconfiguredOtpNotifier),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn log_notifier_ok() {
        LogOtpNotifier
            .send_otp("a@b.co", "123456")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn unconfigured_fails() {
        let err = UnconfiguredOtpNotifier
            .send_otp("a@b.co", "123456")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Internal);
    }
}
