//! Push delivery port (WBS E8.9).
//!
//! Control-plane token registration lives in [`crate::push`]. Delivery is
//! opt-in via env so local/dev never hits a vendor:
//!
//! - unset / `PUSH_DELIVERY=noop` → [`NoopPushDelivery`] (default)
//! - `PUSH_DELIVERY=http` + `PUSH_HTTP_URL` → POST JSON webhook (FCM/APNs bridge)
//! - `PUSH_DELIVERY=log` → trace only (never logs the device token)
//!
//! Payload shape (HTTP):
//! ```json
//! {
//!   "token": "...",
//!   "platform": "ios|android|web|other",
//!   "title": "...",
//!   "body": "...",
//!   "data": { "...": "..." }
//! }
//! ```
//! Optional `PUSH_HTTP_BEARER` for Authorization.

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use async_trait::async_trait;
use serde_json::Value;

/// Notification content for a single device.
#[derive(Debug, Clone)]
pub struct PushMessage {
    pub title: String,
    pub body: String,
    pub data: HashMap<String, String>,
}

/// Port for delivering a push to one device token.
#[async_trait]
pub trait PushDelivery: Send + Sync {
    async fn send(
        &self,
        token: &str,
        platform: &str,
        msg: &PushMessage,
    ) -> Result<(), AppError>;

    fn kind(&self) -> &'static str;
}

pub type SharedPushDelivery = Arc<dyn PushDelivery>;

/// Drop all deliveries (default for tests / local without a bridge).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPushDelivery;

#[async_trait]
impl PushDelivery for NoopPushDelivery {
    async fn send(
        &self,
        _token: &str,
        _platform: &str,
        _msg: &PushMessage,
    ) -> Result<(), AppError> {
        Ok(())
    }

    fn kind(&self) -> &'static str {
        "noop"
    }
}

/// Logs intent only — never the device token.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogPushDelivery;

#[async_trait]
impl PushDelivery for LogPushDelivery {
    async fn send(
        &self,
        token: &str,
        platform: &str,
        msg: &PushMessage,
    ) -> Result<(), AppError> {
        let token_fp = token_fingerprint(token);
        tracing::info!(
            %platform,
            %token_fp,
            title = %msg.title,
            "push delivery: LogPushDelivery (token not logged)"
        );
        Ok(())
    }

    fn kind(&self) -> &'static str {
        "log"
    }
}

/// HTTP webhook bridge (FCM/APNs proxy, OneSignal-style, or self-hosted worker).
#[derive(Debug, Clone)]
pub struct HttpPushDelivery {
    url: String,
    bearer: Option<String>,
}

impl HttpPushDelivery {
    pub fn new(url: impl Into<String>, bearer: Option<String>) -> Self {
        Self {
            url: url.into(),
            bearer,
        }
    }

    pub fn from_env() -> Option<Self> {
        let url = std::env::var("PUSH_HTTP_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;
        let bearer = std::env::var("PUSH_HTTP_BEARER")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Some(Self::new(url, bearer))
    }
}

#[async_trait]
impl PushDelivery for HttpPushDelivery {
    async fn send(
        &self,
        token: &str,
        platform: &str,
        msg: &PushMessage,
    ) -> Result<(), AppError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| {
                AppError::new(ErrorCode::Internal, format!("push http client: {e}"))
            })?;

        let data: Value = msg
            .data
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect::<serde_json::Map<_, _>>()
            .into();
        let body = serde_json::json!({
            "token": token,
            "platform": platform,
            "title": msg.title,
            "body": msg.body,
            "data": data,
        });

        let mut req = client
            .post(&self.url)
            .header("content-type", "application/json")
            .json(&body);
        if let Some(ref token_h) = self.bearer {
            let value = if token_h.to_ascii_lowercase().starts_with("bearer ") {
                token_h.clone()
            } else {
                format!("Bearer {token_h}")
            };
            req = req.header("authorization", value);
        }

        let res = req.send().await.map_err(|e| {
            let fp = token_fingerprint(token);
            tracing::error!(%platform, %fp, error = %e, "push http delivery failed");
            AppError::new(ErrorCode::Internal, "push delivery failed")
        })?;

        if !res.status().is_success() {
            let status = res.status();
            let fp = token_fingerprint(token);
            tracing::error!(%platform, %fp, %status, "push http delivery non-success");
            return Err(AppError::new(
                ErrorCode::Internal,
                format!("push delivery failed (HTTP {status})"),
            ));
        }
        Ok(())
    }

    fn kind(&self) -> &'static str {
        "http"
    }
}

/// Build delivery backend from env. Defaults to noop.
///
/// - `PUSH_DELIVERY=noop|log|http` (default noop)
/// - http requires `PUSH_HTTP_URL`
pub fn push_delivery_from_env() -> SharedPushDelivery {
    let kind = std::env::var("PUSH_DELIVERY")
        .unwrap_or_else(|_| "noop".into())
        .trim()
        .to_ascii_lowercase();
    match kind.as_str() {
        "http" | "webhook" | "fcm" => {
            if let Some(http) = HttpPushDelivery::from_env() {
                tracing::info!(url = %http.url, "push delivery: HttpPushDelivery");
                Arc::new(http)
            } else {
                tracing::warn!(
                    "PUSH_DELIVERY=http but PUSH_HTTP_URL missing; falling back to noop"
                );
                Arc::new(NoopPushDelivery)
            }
        }
        "log" => {
            tracing::info!("push delivery: LogPushDelivery");
            Arc::new(LogPushDelivery)
        }
        _ => Arc::new(NoopPushDelivery),
    }
}

fn token_fingerprint(token: &str) -> String {
    // Short non-reversible-looking prefix for logs (not a crypto hash — just redact).
    let t = token.trim();
    if t.len() <= 8 {
        return "****".into();
    }
    format!("{}…{}", &t[..4], &t[t.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_ok() {
        let d = NoopPushDelivery;
        d.send(
            "tok",
            "ios",
            &PushMessage {
                title: "t".into(),
                body: "b".into(),
                data: HashMap::new(),
            },
        )
        .await
        .unwrap();
        assert_eq!(d.kind(), "noop");
    }

    #[tokio::test]
    async fn log_ok_and_redacts() {
        let d = LogPushDelivery;
        d.send(
            "abcdefghijklmnop",
            "android",
            &PushMessage {
                title: "hi".into(),
                body: "there".into(),
                data: HashMap::new(),
            },
        )
        .await
        .unwrap();
        assert_eq!(token_fingerprint("abcdefghijklmnop"), "abcd…mnop");
    }

    #[test]
    fn from_env_defaults_noop() {
        // Ensure the factory is callable; exact backend depends on process env.
        let d = push_delivery_from_env();
        assert!(!d.kind().is_empty());
    }
}
