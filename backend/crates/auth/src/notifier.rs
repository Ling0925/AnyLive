//! Delivery ports for OTP / notifications.
//!
//! Production must wire a real [`OtpNotifier`] (log, SMTP-via-HTTP, SMS, etc.).
//! Local/dev may use [`LogOtpNotifier`] or [`NoopOtpNotifier`] only when
//! fixed OTP is explicitly enabled.
//!
//! ## HTTP mailer (`OTP_NOTIFIER=smtp` or `http`)
//!
//! Posts JSON to `OTP_HTTP_URL` (or `OTP_SMTP_WEBHOOK_URL`):
//! ```json
//! { "to": "user@example.com", "subject": "...", "text": "...", "code": "......" }
//! ```
//! Optional `OTP_HTTP_BEARER` / `OTP_HTTP_AUTHORIZATION` for the Authorization header.
//! Optional `OTP_HTTP_FROM` is included as `"from"` when set.
//!
//! This is the standard integration shape for Mailgun/SendGrid/Resend-style
//! proxies and self-hosted SMTP bridges. Native SMTP (lettre) can be added later.

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

/// HTTP webhook mailer: POST JSON to a configurable URL (SMTP bridge / ESP).
#[derive(Debug, Clone)]
pub struct HttpOtpNotifier {
    url: String,
    bearer: Option<String>,
    from: Option<String>,
    subject: String,
}

impl HttpOtpNotifier {
    pub fn new(
        url: impl Into<String>,
        bearer: Option<String>,
        from: Option<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            bearer,
            from,
            subject: subject.into(),
        }
    }

    /// Build from env. Returns `None` if URL is missing.
    ///
    /// Env:
    /// - `OTP_HTTP_URL` or `OTP_SMTP_WEBHOOK_URL` (required)
    /// - `OTP_HTTP_BEARER` or `OTP_HTTP_AUTHORIZATION` (optional)
    /// - `OTP_HTTP_FROM` (optional)
    /// - `OTP_HTTP_SUBJECT` (optional, default "Your AnyLive login code")
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("OTP_HTTP_URL")
            .or_else(|_| std::env::var("OTP_SMTP_WEBHOOK_URL"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;
        let bearer = std::env::var("OTP_HTTP_BEARER")
            .or_else(|_| std::env::var("OTP_HTTP_AUTHORIZATION"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let from = std::env::var("OTP_HTTP_FROM")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let subject = std::env::var("OTP_HTTP_SUBJECT")
            .unwrap_or_else(|_| "Your AnyLive login code".into());
        Some(Self::new(url, bearer, from, subject))
    }
}

#[async_trait]
impl OtpNotifier for HttpOtpNotifier {
    async fn send_otp(&self, email: &str, code: &str) -> Result<(), AppError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| {
                AppError::new(ErrorCode::Internal, format!("otp http client: {e}"))
            })?;

        let text = format!(
            "Your AnyLive verification code is: {code}\n\nThis code expires shortly. If you did not request it, ignore this email."
        );
        let mut body = serde_json::json!({
            "to": email,
            "subject": self.subject,
            "text": text,
            // ESP bridges may map `code` into a template; still never log it here.
            "code": code,
            "purpose": "login_otp",
        });
        if let Some(ref from) = self.from {
            body["from"] = serde_json::Value::String(from.clone());
        }

        let mut req = client
            .post(&self.url)
            .header("content-type", "application/json")
            .json(&body);
        if let Some(ref token) = self.bearer {
            // Accept either raw token or already "Bearer xxx".
            let value = if token.to_ascii_lowercase().starts_with("bearer ") {
                token.clone()
            } else {
                format!("Bearer {token}")
            };
            req = req.header("authorization", value);
        }

        let res = req.send().await.map_err(|e| {
            tracing::error!(%email, error = %e, "otp http delivery failed");
            AppError::new(ErrorCode::Internal, "OTP delivery failed")
        })?;

        if !res.status().is_success() {
            let status = res.status();
            // Do not include response body (may echo code).
            tracing::error!(%email, %status, "otp http delivery non-success");
            return Err(AppError::new(
                ErrorCode::Internal,
                format!("OTP delivery failed (HTTP {status})"),
            ));
        }
        tracing::info!(%email, "otp delivery: HttpOtpNotifier ok");
        Ok(())
    }
}

/// Build notifier from env:
/// - `OTP_NOTIFIER=log` → [`LogOtpNotifier`]
/// - `OTP_NOTIFIER=noop` → [`NoopOtpNotifier`]
/// - `OTP_NOTIFIER=smtp` | `http` → [`HttpOtpNotifier`] (requires URL)
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
        "smtp" | "http" | "webhook" => match HttpOtpNotifier::from_env() {
            Some(n) => Arc::new(n),
            None => {
                tracing::error!(
                    "OTP_NOTIFIER=smtp|http requires OTP_HTTP_URL or OTP_SMTP_WEBHOOK_URL"
                );
                Arc::new(UnconfiguredOtpNotifier)
            }
        },
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

    #[test]
    fn http_from_env_requires_url() {
        // Isolation: clear vars that might exist in the developer environment.
        std::env::remove_var("OTP_HTTP_URL");
        std::env::remove_var("OTP_SMTP_WEBHOOK_URL");
        assert!(HttpOtpNotifier::from_env().is_none());
        std::env::set_var("OTP_HTTP_URL", "https://mail.example/send");
        std::env::set_var("OTP_HTTP_BEARER", "tok");
        let n = HttpOtpNotifier::from_env().expect("url set");
        assert_eq!(n.url, "https://mail.example/send");
        assert_eq!(n.bearer.as_deref(), Some("tok"));
        std::env::remove_var("OTP_HTTP_URL");
        std::env::remove_var("OTP_HTTP_BEARER");
    }

    #[test]
    fn otp_notifier_from_env_log() {
        std::env::set_var("OTP_NOTIFIER", "log");
        // Type erasure — just ensure it constructs.
        let _ = otp_notifier_from_env();
        std::env::remove_var("OTP_NOTIFIER");
    }
}
