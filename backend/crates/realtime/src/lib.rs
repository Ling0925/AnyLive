//! Realtime plane helpers: chat messages + Centrifugo connection tokens + publish.
//!
//! Actual WebSocket fan-out is Centrifugo (or in-memory bus for tests).
//! Optional NATS domain events: [`nats`].

mod nats;
pub use nats::{
    gift_sent_nats_event, nats_publisher_from_env, NatsPublisher, NoopNatsPublisher,
    RecordingNatsPublisher, TcpNatsPublisher, SUBJECT_GIFT_SENT,
};

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, UserId};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Max chat body length.
pub const MAX_CHAT_LEN: usize = 500;

/// Default max messages per window for chat rate limiting.
pub const DEFAULT_CHAT_RATE_LIMIT: usize = 5;

/// Default sliding window for chat rate limiting.
pub const DEFAULT_CHAT_RATE_WINDOW: StdDuration = StdDuration::from_secs(10);

/// Realtime event type for chat messages on room channels.
pub const EVENT_CHAT_MESSAGE: &str = "chat.message";

/// Realtime event type for gift orders fan-out on room channels.
pub const EVENT_GIFT_SENT: &str = "gift.sent";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: Uuid,
    pub room_id: RoomId,
    pub sender_id: UserId,
    pub sender_name: String,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
}

/// Envelope published to Centrifugo channels (chat; gift uses [`gift_envelope`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageEnvelope {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: ChatMessagePayload,
}

/// JSON-stable chat payload (string ids for client compatibility).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessagePayload {
    pub id: String,
    pub room_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub body: String,
    pub created_at: String,
}

/// JSON-stable gift fan-out payload (string ids for client compatibility).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GiftSentPayload {
    pub order_id: String,
    pub room_id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub gift_id: String,
    pub count: u32,
    pub total_coins: i64,
}

impl From<&ChatMessage> for ChatMessagePayload {
    fn from(m: &ChatMessage) -> Self {
        Self {
            id: m.id.to_string(),
            room_id: m.room_id.0.to_string(),
            sender_id: m.sender_id.0.to_string(),
            sender_name: m.sender_name.clone(),
            body: m.body.clone(),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

impl MessageEnvelope {
    pub fn chat_message(msg: &ChatMessage) -> Self {
        Self {
            event_type: EVENT_CHAT_MESSAGE.to_string(),
            payload: ChatMessagePayload::from(msg),
        }
    }

    pub fn room_channel(room_id: RoomId) -> String {
        format!("room:{}", room_id.0)
    }

    pub fn to_value(&self) -> Result<serde_json::Value, AppError> {
        serde_json::to_value(self)
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("envelope serialize: {e}")))
    }
}

/// Build a Centrifugo-ready gift.sent envelope (`type` + `payload`).
pub fn gift_envelope(
    order_id: Uuid,
    room_id: Uuid,
    sender_id: UserId,
    receiver_id: UserId,
    gift_id: Uuid,
    count: u32,
    total_coins: i64,
) -> serde_json::Value {
    let payload = GiftSentPayload {
        order_id: order_id.to_string(),
        room_id: room_id.to_string(),
        sender_id: sender_id.0.to_string(),
        receiver_id: receiver_id.0.to_string(),
        gift_id: gift_id.to_string(),
        count,
        total_coins,
    };
    serde_json::json!({
        "type": EVENT_GIFT_SENT,
        "payload": payload,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentrifugoToken {
    pub token: String,
    pub expires_in: i64,
    pub channels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CentrifugoConfig {
    pub token_secret: String,
    pub token_ttl_secs: i64,
}

impl Default for CentrifugoConfig {
    fn default() -> Self {
        Self {
            token_secret: std::env::var("CENTRIFUGO_TOKEN_SECRET")
                .unwrap_or_else(|_| "anylive-dev-token-secret-change-me".into()),
            token_ttl_secs: 3600,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CentrifugoClaims {
    sub: String,
    exp: i64,
    iat: i64,
    channels: Vec<String>,
}

/// Issue a Centrifugo client JWT for room subscription.
pub fn issue_centrifugo_token(
    cfg: &CentrifugoConfig,
    user_id: UserId,
    room_id: RoomId,
) -> Result<CentrifugoToken, AppError> {
    if cfg.token_secret.len() < 16 {
        return Err(AppError::new(
            ErrorCode::Internal,
            "centrifugo token secret too short",
        ));
    }
    let now = Utc::now();
    let exp = now + Duration::seconds(cfg.token_ttl_secs);
    let channel = MessageEnvelope::room_channel(room_id);
    let claims = CentrifugoClaims {
        sub: user_id.0.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        channels: vec![channel.clone()],
    };
    let mut header = Header::new(Algorithm::HS256);
    header.typ = Some("JWT".into());
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(cfg.token_secret.as_bytes()),
    )
    .map_err(|e| AppError::new(ErrorCode::Internal, format!("jwt encode: {e}")))?;
    Ok(CentrifugoToken {
        token,
        expires_in: cfg.token_ttl_secs,
        channels: vec![channel],
    })
}

/// Port for publishing events into Centrifugo (or a no-op / test double).
#[async_trait]
pub trait CentrifugoPublisher: Send + Sync {
    async fn publish(&self, channel: &str, data: serde_json::Value) -> Result<(), AppError>;
}

/// No-op publisher used when Centrifugo is not configured (memory/offline).
#[derive(Debug, Clone, Default)]
pub struct NoopCentrifugoPublisher;

impl NoopCentrifugoPublisher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CentrifugoPublisher for NoopCentrifugoPublisher {
    async fn publish(&self, _channel: &str, _data: serde_json::Value) -> Result<(), AppError> {
        Ok(())
    }
}

/// HTTP publisher against Centrifugo server API (`POST {url}/api`).
#[derive(Debug, Clone)]
pub struct HttpCentrifugoPublisher {
    client: reqwest::Client,
    api_url: String,
    api_key: String,
}

impl HttpCentrifugoPublisher {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base = base_url.into().trim_end_matches('/').to_string();
        Self {
            client: reqwest::Client::new(),
            api_url: format!("{base}/api"),
            api_key: api_key.into(),
        }
    }

    /// Build from `CENTRIFUGO_URL` + `CENTRIFUGO_API_KEY` when both are set.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("CENTRIFUGO_URL").ok().filter(|s| !s.is_empty())?;
        let key = std::env::var("CENTRIFUGO_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(Self::new(url, key))
    }
}

#[derive(Debug, Serialize)]
struct CentrifugoApiRequest<'a> {
    method: &'static str,
    params: CentrifugoPublishParams<'a>,
}

#[derive(Debug, Serialize)]
struct CentrifugoPublishParams<'a> {
    channel: &'a str,
    data: serde_json::Value,
}

#[async_trait]
impl CentrifugoPublisher for HttpCentrifugoPublisher {
    async fn publish(&self, channel: &str, data: serde_json::Value) -> Result<(), AppError> {
        let body = CentrifugoApiRequest {
            method: "publish",
            params: CentrifugoPublishParams { channel, data },
        };
        let res = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("apikey {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::new(
                    ErrorCode::Internal,
                    format!("centrifugo publish request: {e}"),
                )
            })?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        // Centrifugo often returns HTTP 200 even on command errors; inspect body.
        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::Internal,
                format!("centrifugo publish HTTP {status}: {text}"),
            ));
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(err) = v.get("error") {
                if !err.is_null() {
                    return Err(AppError::new(
                        ErrorCode::Internal,
                        format!("centrifugo publish error: {err}"),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Build the process-wide publisher: HTTP when env is set, otherwise no-op.
pub fn publisher_from_env() -> Arc<dyn CentrifugoPublisher> {
    match HttpCentrifugoPublisher::from_env() {
        Some(http) => {
            tracing::info!("centrifugo HTTP publisher enabled");
            Arc::new(http)
        }
        None => {
            tracing::info!("centrifugo HTTP publisher disabled (memory/noop)");
            Arc::new(NoopCentrifugoPublisher::new())
        }
    }
}

/// Sliding-window rate limiter: max N events per user within a time window.
///
/// Pure in-memory; suitable for single-process API. Prunes expired timestamps
/// on each check so the map does not grow unbounded per active user.
#[derive(Debug, Clone)]
pub struct ChatRateLimiter {
    max: usize,
    window: StdDuration,
    hits: Arc<Mutex<HashMap<Uuid, VecDeque<Instant>>>>,
}

impl Default for ChatRateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_CHAT_RATE_LIMIT, DEFAULT_CHAT_RATE_WINDOW)
    }
}

impl ChatRateLimiter {
    pub fn new(max: usize, window: StdDuration) -> Self {
        Self {
            max: max.max(1),
            window,
            hits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Pure check/record against an arbitrary clock instant (unit-testable).
    ///
    /// Returns `true` if the event is allowed (and records it), `false` if
    /// the user is over the limit.
    pub fn try_acquire_at(
        hits: &mut HashMap<Uuid, VecDeque<Instant>>,
        user_id: Uuid,
        max: usize,
        window: StdDuration,
        now: Instant,
    ) -> bool {
        let queue = hits.entry(user_id).or_default();
        // Drop timestamps outside the sliding window.
        while queue
            .front()
            .is_some_and(|t| now.duration_since(*t) >= window)
        {
            queue.pop_front();
        }
        if queue.len() >= max {
            return false;
        }
        queue.push_back(now);
        true
    }

    /// Check and record a message attempt for `user_id`.
    ///
    /// Returns `Ok(())` when allowed, or `Err(AppError)` with
    /// [`ErrorCode::RateLimited`] when the limit is exceeded.
    pub async fn check(&self, user_id: UserId) -> Result<(), AppError> {
        let mut g = self.hits.lock().await;
        if Self::try_acquire_at(&mut g, user_id.0, self.max, self.window, Instant::now()) {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::RateLimited,
                "chat rate limit exceeded",
            ))
        }
    }
}

/// In-memory chat log + recent messages (tests / offline).
#[derive(Clone, Default)]
pub struct MemoryChatBus {
    inner: Arc<Mutex<HashMap<Uuid, Vec<ChatMessage>>>>,
}

impl MemoryChatBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn post(
        &self,
        room_id: RoomId,
        sender_id: UserId,
        sender_name: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<ChatMessage, AppError> {
        let body = body.into().trim().to_string();
        if body.is_empty() || body.len() > MAX_CHAT_LEN {
            return Err(AppError::validation("invalid chat body"));
        }
        // basic spam: reject control chars except newline
        if body.chars().any(|c| c.is_control() && c != '\n') {
            return Err(AppError::validation("chat body has control characters"));
        }
        let msg = ChatMessage {
            id: Uuid::new_v4(),
            room_id,
            sender_id,
            sender_name: sender_name.into(),
            body,
            created_at: Utc::now(),
        };
        let mut g = self.inner.lock().await;
        let list = g.entry(room_id.0).or_default();
        list.push(msg.clone());
        // keep last 200
        if list.len() > 200 {
            let drain = list.len() - 200;
            list.drain(0..drain);
        }
        Ok(msg)
    }

    pub async fn recent(&self, room_id: RoomId, limit: usize) -> Vec<ChatMessage> {
        let g = self.inner.lock().await;
        let list = g.get(&room_id.0).cloned().unwrap_or_default();
        let limit = limit.clamp(1, 100);
        list.into_iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

/// Test double that records publish calls.
#[derive(Clone, Default)]
pub struct RecordingCentrifugoPublisher {
    pub published: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

impl RecordingCentrifugoPublisher {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn snapshot(&self) -> Vec<(String, serde_json::Value)> {
        self.published.lock().await.clone()
    }
}

#[async_trait]
impl CentrifugoPublisher for RecordingCentrifugoPublisher {
    async fn publish(&self, channel: &str, data: serde_json::Value) -> Result<(), AppError> {
        self.published
            .lock()
            .await
            .push((channel.to_string(), data));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_up_to_max_then_blocks() {
        let mut hits: HashMap<Uuid, VecDeque<Instant>> = HashMap::new();
        let user = Uuid::new_v4();
        let other = Uuid::new_v4();
        let max = 5;
        let window = StdDuration::from_secs(10);
        let t0 = Instant::now();

        for i in 0..max {
            assert!(
                ChatRateLimiter::try_acquire_at(&mut hits, user, max, window, t0),
                "message {i} should be allowed"
            );
        }
        // 6th within window is blocked
        assert!(!ChatRateLimiter::try_acquire_at(
            &mut hits, user, max, window, t0
        ));
        // other users are independent
        assert!(ChatRateLimiter::try_acquire_at(
            &mut hits, other, max, window, t0
        ));
    }

    #[test]
    fn rate_limiter_prunes_old_entries_and_recovers() {
        let mut hits: HashMap<Uuid, VecDeque<Instant>> = HashMap::new();
        let user = Uuid::new_v4();
        let max = 5;
        let window = StdDuration::from_secs(10);
        let t0 = Instant::now();

        for _ in 0..max {
            assert!(ChatRateLimiter::try_acquire_at(
                &mut hits, user, max, window, t0
            ));
        }
        assert!(!ChatRateLimiter::try_acquire_at(
            &mut hits, user, max, window, t0
        ));

        // After window elapses, oldest entries prune and a new message is allowed.
        let t1 = t0 + window + StdDuration::from_millis(1);
        assert!(ChatRateLimiter::try_acquire_at(
            &mut hits, user, max, window, t1
        ));
        // Queue should only hold the single fresh hit.
        assert_eq!(hits.get(&user).map(|q| q.len()), Some(1));
    }

    #[tokio::test]
    async fn rate_limiter_check_returns_rate_limited_code() {
        let limiter = ChatRateLimiter::new(2, StdDuration::from_secs(10));
        let user = UserId::new();
        limiter.check(user).await.unwrap();
        limiter.check(user).await.unwrap();
        let err = limiter.check(user).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::RateLimited);
        assert!(err.message.contains("rate limit"));
    }

    #[test]
    fn centrifugo_token_has_room_channel() {
        let cfg = CentrifugoConfig {
            token_secret: "dev-secret-at-least-16".into(),
            token_ttl_secs: 60,
        };
        let room = RoomId::new();
        let user = UserId::new();
        let t = issue_centrifugo_token(&cfg, user, room).unwrap();
        assert_eq!(t.channels, vec![format!("room:{}", room.0)]);
        assert!(!t.token.is_empty());
        assert_eq!(t.expires_in, 60);
    }

    #[tokio::test]
    async fn chat_post_and_recent() {
        let bus = MemoryChatBus::new();
        let room = RoomId::new();
        let u = UserId::new();
        bus.post(room, u, "Alice", "hello").await.unwrap();
        bus.post(room, u, "Alice", "world").await.unwrap();
        let recent = bus.recent(room, 10).await;
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].body, "hello");
        assert_eq!(recent[1].body, "world");
    }

    #[tokio::test]
    async fn reject_empty_chat() {
        let bus = MemoryChatBus::new();
        let err = bus
            .post(RoomId::new(), UserId::new(), "x", "   ")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn message_envelope_serializes_chat_type() {
        let room = RoomId::new();
        let user = UserId::new();
        let id = Uuid::new_v4();
        let created = Utc::now();
        let msg = ChatMessage {
            id,
            room_id: room,
            sender_id: user,
            sender_name: "Alice".into(),
            body: "hello live".into(),
            created_at: created,
        };
        let env = MessageEnvelope::chat_message(&msg);
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["type"], "chat.message");
        assert_eq!(json["payload"]["id"], id.to_string());
        assert_eq!(json["payload"]["room_id"], room.0.to_string());
        assert_eq!(json["payload"]["sender_id"], user.0.to_string());
        assert_eq!(json["payload"]["sender_name"], "Alice");
        assert_eq!(json["payload"]["body"], "hello live");
        assert_eq!(json["payload"]["created_at"], created.to_rfc3339());
        // round-trip
        let back: MessageEnvelope = serde_json::from_value(json).unwrap();
        assert_eq!(back.event_type, EVENT_CHAT_MESSAGE);
        assert_eq!(back.payload.body, "hello live");
    }

    #[test]
    fn gift_envelope_serializes_gift_sent_type() {
        let order_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let sender = UserId::new();
        let receiver = UserId::new();
        let gift_id = Uuid::new_v4();
        let json = gift_envelope(order_id, room_id, sender, receiver, gift_id, 3, 300);
        assert_eq!(json["type"], "gift.sent");
        assert_eq!(json["type"], EVENT_GIFT_SENT);
        assert_eq!(json["payload"]["order_id"], order_id.to_string());
        assert_eq!(json["payload"]["room_id"], room_id.to_string());
        assert_eq!(json["payload"]["sender_id"], sender.0.to_string());
        assert_eq!(json["payload"]["receiver_id"], receiver.0.to_string());
        assert_eq!(json["payload"]["gift_id"], gift_id.to_string());
        assert_eq!(json["payload"]["count"], 3);
        assert_eq!(json["payload"]["total_coins"], 300);
        // payload round-trip
        let payload: GiftSentPayload =
            serde_json::from_value(json["payload"].clone()).unwrap();
        assert_eq!(payload.count, 3);
        assert_eq!(payload.total_coins, 300);
        assert_eq!(payload.order_id, order_id.to_string());
    }

    #[tokio::test]
    async fn recording_publisher_captures_gift_envelope() {
        let pubr = RecordingCentrifugoPublisher::new();
        let room = RoomId::new();
        let order_id = Uuid::new_v4();
        let sender = UserId::new();
        let receiver = UserId::new();
        let gift_id = Uuid::new_v4();
        let data = gift_envelope(order_id, room.0, sender, receiver, gift_id, 1, 10);
        let channel = MessageEnvelope::room_channel(room);
        pubr.publish(&channel, data).await.unwrap();
        let snap = pubr.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, format!("room:{}", room.0));
        assert_eq!(snap[0].1["type"], "gift.sent");
        assert_eq!(snap[0].1["payload"]["order_id"], order_id.to_string());
        assert_eq!(snap[0].1["payload"]["count"], 1);
        assert_eq!(snap[0].1["payload"]["total_coins"], 10);
    }

    #[tokio::test]
    async fn recording_publisher_captures_envelope() {
        let pubr = RecordingCentrifugoPublisher::new();
        let room = RoomId::new();
        let msg = ChatMessage {
            id: Uuid::new_v4(),
            room_id: room,
            sender_id: UserId::new(),
            sender_name: "Bob".into(),
            body: "hi".into(),
            created_at: Utc::now(),
        };
        let env = MessageEnvelope::chat_message(&msg);
        let channel = MessageEnvelope::room_channel(room);
        pubr.publish(&channel, env.to_value().unwrap())
            .await
            .unwrap();
        let snap = pubr.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, format!("room:{}", room.0));
        assert_eq!(snap[0].1["type"], "chat.message");
        assert_eq!(snap[0].1["payload"]["body"], "hi");
    }

    #[tokio::test]
    async fn noop_publisher_ok() {
        let p = NoopCentrifugoPublisher::new();
        p.publish("room:x", serde_json::json!({"ok": true}))
            .await
            .unwrap();
    }

    #[test]
    fn http_publisher_from_env_none_without_vars() {
        // Ensure missing env yields None (do not set vars in unit tests).
        // If CI injects them, skip — production path still covered by Option.
        let has_url = std::env::var("CENTRIFUGO_URL").ok().filter(|s| !s.is_empty());
        let has_key = std::env::var("CENTRIFUGO_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        if has_url.is_none() || has_key.is_none() {
            assert!(HttpCentrifugoPublisher::from_env().is_none());
        }
    }

    #[test]
    fn http_publisher_builds_api_url() {
        let p = HttpCentrifugoPublisher::new("http://localhost:8001/", "secret-key");
        assert_eq!(p.api_url, "http://localhost:8001/api");
        assert_eq!(p.api_key, "secret-key");
    }

    #[test]
    fn centrifugo_error_body_is_detected() {
        // Mirrors the body check in HttpCentrifugoPublisher::publish.
        let text = r#"{"error":{"code":108,"message":"not available"}}"#;
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        let err = v.get("error").unwrap();
        assert!(!err.is_null());
        let ok_text = r#"{"result":{}}"#;
        let ok: serde_json::Value = serde_json::from_str(ok_text).unwrap();
        assert!(ok.get("error").is_none() || ok.get("error").unwrap().is_null());
    }
}
