//! Realtime plane helpers: chat messages + Centrifugo connection tokens.
//!
//! Actual WebSocket fan-out is Centrifugo (or in-memory bus for tests).

use std::collections::HashMap;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, UserId};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Max chat body length.
pub const MAX_CHAT_LEN: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: Uuid,
    pub room_id: RoomId,
    pub sender_id: UserId,
    pub sender_name: String,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
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
    let channel = format!("room:{}", room_id.0);
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
        list.into_iter().rev().take(limit).collect::<Vec<_>>().into_iter().rev().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
