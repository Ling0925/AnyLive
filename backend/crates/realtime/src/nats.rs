//! NATS event publish (WBS E1.3 / E5.3).
//!
//! Optional fire-and-forget publisher. When `NATS_URL` is unset the process uses
//! [`NoopNatsPublisher`]. With URL set, a minimal NATS client opens a TCP
//! connection and issues `PUB gift.sent` (JSON envelope). Failures are logged
//! and never fail the gift debit path.

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// Subject for gift.sent domain events (schema v1).
pub const SUBJECT_GIFT_SENT: &str = "gift.sent";

/// Port for publishing domain events to NATS (or a no-op / test double).
#[async_trait]
pub trait NatsPublisher: Send + Sync {
    async fn publish(&self, subject: &str, data: Value) -> Result<(), AppError>;
}

/// No-op used when NATS is not configured.
#[derive(Debug, Clone, Default)]
pub struct NoopNatsPublisher;

impl NoopNatsPublisher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NatsPublisher for NoopNatsPublisher {
    async fn publish(&self, _subject: &str, _data: Value) -> Result<(), AppError> {
        Ok(())
    }
}

/// Records published messages for tests.
#[derive(Debug, Default)]
pub struct RecordingNatsPublisher {
    pub messages: Mutex<Vec<(String, Value)>>,
}

impl RecordingNatsPublisher {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
        }
    }

    pub async fn len(&self) -> usize {
        self.messages.lock().await.len()
    }
}

#[async_trait]
impl NatsPublisher for RecordingNatsPublisher {
    async fn publish(&self, subject: &str, data: Value) -> Result<(), AppError> {
        self.messages
            .lock()
            .await
            .push((subject.to_string(), data));
        Ok(())
    }
}

/// Minimal NATS TCP publisher (CONNECT + PUB). Reconnects per publish to keep
/// the implementation dependency-free (no async-nats crate).
#[derive(Debug, Clone)]
pub struct TcpNatsPublisher {
    /// host:port (nats default 4222). `nats://` scheme stripped.
    addr: String,
}

impl TcpNatsPublisher {
    pub fn new(url: impl Into<String>) -> Self {
        let raw = url.into();
        let addr = raw
            .trim()
            .trim_start_matches("nats://")
            .trim_start_matches("tls://")
            .trim_end_matches('/')
            .to_string();
        Self { addr }
    }

    pub fn from_env() -> Option<Self> {
        let url = std::env::var("NATS_URL").ok().filter(|s| !s.is_empty())?;
        Some(Self::new(url))
    }
}

#[async_trait]
impl NatsPublisher for TcpNatsPublisher {
    async fn publish(&self, subject: &str, data: Value) -> Result<(), AppError> {
        if subject.is_empty()
            || subject.len() > 200
            || !subject
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            return Err(AppError::validation("invalid nats subject"));
        }
        let payload = serde_json::to_vec(&data).map_err(|e| {
            AppError::new(ErrorCode::Internal, format!("nats payload serialize: {e}"))
        })?;
        let mut stream = TcpStream::connect(&self.addr).await.map_err(|e| {
            AppError::new(
                ErrorCode::Internal,
                format!("nats connect {}: {e}", self.addr),
            )
        })?;
        // Read INFO line (best-effort, 2s).
        let mut buf = [0u8; 512];
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            stream.read(&mut buf),
        )
        .await;

        let connect = b"CONNECT {\"verbose\":false,\"pedantic\":false,\"name\":\"anylive-api\"}\r\n";
        stream.write_all(connect).await.map_err(|e| {
            AppError::new(ErrorCode::Internal, format!("nats connect write: {e}"))
        })?;

        let header = format!("PUB {subject} {}\r\n", payload.len());
        stream
            .write_all(header.as_bytes())
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("nats pub header: {e}")))?;
        stream
            .write_all(&payload)
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("nats pub body: {e}")))?;
        stream
            .write_all(b"\r\n")
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("nats pub trailer: {e}")))?;
        // Best-effort flush + short read for +OK / -ERR.
        let _ = stream.flush().await;
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            stream.read(&mut buf),
        )
        .await;
        Ok(())
    }
}

/// Build process-wide NATS publisher: TCP when `NATS_URL` set, otherwise no-op.
pub fn nats_publisher_from_env() -> Arc<dyn NatsPublisher> {
    match TcpNatsPublisher::from_env() {
        Some(tcp) => {
            tracing::info!(addr = %tcp.addr, "nats TCP publisher enabled");
            Arc::new(tcp)
        }
        None => {
            tracing::info!("nats publisher disabled (NATS_URL unset)");
            Arc::new(NoopNatsPublisher::new())
        }
    }
}

/// Build gift.sent NATS envelope (schema v1 — stable string ids).
pub fn gift_sent_nats_event(
    order_id: uuid::Uuid,
    room_id: uuid::Uuid,
    sender_id: uuid::Uuid,
    receiver_id: uuid::Uuid,
    gift_id: uuid::Uuid,
    count: u32,
    total_coins: i64,
) -> Value {
    serde_json::json!({
        "schema": "anylive.gift.sent.v1",
        "type": "gift.sent",
        "payload": {
            "order_id": order_id.to_string(),
            "room_id": room_id.to_string(),
            "sender_id": sender_id.to_string(),
            "receiver_id": receiver_id.to_string(),
            "gift_id": gift_id.to_string(),
            "count": count,
            "total_coins": total_coins,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recording_publisher_captures() {
        let p = RecordingNatsPublisher::new();
        let ev = gift_sent_nats_event(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            2,
            10,
        );
        p.publish(SUBJECT_GIFT_SENT, ev.clone()).await.unwrap();
        assert_eq!(p.len().await, 1);
        let msgs = p.messages.lock().await;
        assert_eq!(msgs[0].0, SUBJECT_GIFT_SENT);
        assert_eq!(msgs[0].1["schema"], "anylive.gift.sent.v1");
        assert_eq!(msgs[0].1["type"], "gift.sent");
    }

    #[test]
    fn strips_nats_scheme() {
        let p = TcpNatsPublisher::new("nats://127.0.0.1:4222");
        assert_eq!(p.addr, "127.0.0.1:4222");
    }

    #[test]
    fn gift_event_shape() {
        let id = uuid::Uuid::nil();
        let v = gift_sent_nats_event(id, id, id, id, id, 1, 5);
        assert_eq!(v["payload"]["total_coins"], 5);
    }
}
