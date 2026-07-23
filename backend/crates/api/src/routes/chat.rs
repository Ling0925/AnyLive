//! Chat + realtime token HTTP handlers.

use std::sync::Arc;

use anylive_domain::RoomId;
use anylive_realtime::{issue_centrifugo_token, ChatMessage, MessageEnvelope};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RealtimeTokenBody {
    pub room_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RealtimeTokenResponse {
    pub token: String,
    pub expires_in: i64,
    pub channels: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostMessageBody {
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatMessageDto {
    pub id: String,
    pub room_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub body: String,
    pub created_at: String,
}

impl From<ChatMessage> for ChatMessageDto {
    fn from(m: ChatMessage) -> Self {
        Self {
            id: m.id.to_string(),
            room_id: m.room_id.0.to_string(),
            sender_id: m.sender_id.0.to_string(),
            sender_name: m.sender_name,
            body: m.body,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatListResponse {
    pub items: Vec<ChatMessageDto>,
}

#[derive(Debug, Deserialize)]
pub struct ChatHistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Issue Centrifugo connection token for a room channel.
#[utoipa::path(post, path = "/api/v1/realtime/token", tag = "realtime", security(("bearerAuth" = [])), request_body = RealtimeTokenBody, responses((status = 200, body = RealtimeTokenResponse)))]
pub async fn realtime_token(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<RealtimeTokenBody>,
) -> Result<Json<RealtimeTokenResponse>, ApiError> {
    let room_uuid = Uuid::parse_str(&body.room_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room_id")))?;
    let room_id = RoomId(room_uuid);
    // room must exist
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let token = issue_centrifugo_token(&state.centrifugo, user.user_id, room_id)
        .map_err(ApiError::from)?;
    Ok(Json(RealtimeTokenResponse {
        token: token.token,
        expires_in: token.expires_in,
        channels: token.channels,
    }))
}

/// Post a chat message (also stored for history snapshot).
#[utoipa::path(post, path = "/api/v1/rooms/{id}/messages", tag = "realtime", security(("bearerAuth" = [])), request_body = PostMessageBody, responses((status = 201, body = ChatMessageDto)))]
pub async fn post_message(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<PostMessageBody>,
) -> Result<(axum::http::StatusCode, Json<ChatMessageDto>), ApiError> {
    if state
        .moderation
        .try_is_muted(user.user_id)
        .await
        .map_err(ApiError)?
    {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "user is muted",
        )));
    }
    // Per-user chat rate limit (default 5 messages / 10s).
    state
        .chat_rate_limiter
        .check(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let me = state.auth.me(user.user_id).await.map_err(ApiError::from)?;
    state
        .word_filter
        .check(&body.body)
        .map_err(ApiError::from)?;
    let msg = state
        .chat
        .post(room_id, user.user_id, me.display_name, body.body)
        .await
        .map_err(ApiError::from)?;
    // Fan-out via Centrifugo when configured; Noop otherwise (memory-only).
    // Publish failures are logged but do not fail the REST write — history is source of truth.
    let envelope = MessageEnvelope::chat_message(&msg);
    let channel = MessageEnvelope::room_channel(room_id);
    match envelope.to_value() {
        Ok(data) => {
            if let Err(e) = state.centrifugo_publisher.publish(&channel, data).await {
                tracing::warn!(error = %e, %channel, "centrifugo publish failed");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, %channel, "centrifugo envelope serialize failed");
        }
    }
    Ok((axum::http::StatusCode::CREATED, Json(msg.into())))
}

/// Recent chat history (REST snapshot for reconnect).
#[utoipa::path(get, path = "/api/v1/rooms/{id}/messages", tag = "realtime", responses((status = 200, body = ChatListResponse)))]
pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ChatHistoryQuery>,
) -> Result<Json<ChatListResponse>, ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let items = state.chat.recent(room_id, q.limit).await;
    Ok(Json(ChatListResponse {
        items: items.into_iter().map(ChatMessageDto::from).collect(),
    }))
}
