//! Room online count + likes HTTP handlers (WBS E4.4).

use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::RoomId;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

fn parse_room_id(id: &str) -> Result<RoomId, ApiError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|_| ApiError(AppError::validation("invalid room id")))?;
    Ok(RoomId(uuid))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoomStatsDto {
    pub room_id: String,
    pub online_count: u64,
    pub like_count: u64,
    /// Host recording flag (WBS E3.5 control plane).
    pub recording_enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PresenceHeartbeatResponse {
    pub online_count: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LikeRoomBody {
    /// Optional client idempotency key (reserved; currently unused).
    #[serde(default)]
    #[allow(dead_code)]
    pub client_request_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LikeRoomResponse {
    pub accepted: bool,
    pub like_count: u64,
}

/// Public room stats: online viewers (TTL heartbeats) + cumulative likes.
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{id}/stats",
    tag = "rooms",
    params(("id" = String, Path, description = "Room id")),
    responses((status = 200, body = RoomStatsDto), (status = 404, description = "Room not found"))
)]
pub async fn room_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RoomStatsDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    // Ensure room exists.
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let online_count = state.presence.online_count(room_id).await;
    let like_count = state.presence.like_count(room_id).await;
    let recording_enabled = state.recording.get(room_id).await;
    Ok(Json(RoomStatsDto {
        room_id: room_id.0.to_string(),
        online_count,
        like_count,
        recording_enabled,
    }))
}

/// Authenticated presence heartbeat (client should call every ~15–30s while in room).
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/presence",
    tag = "rooms",
    params(("id" = String, Path, description = "Room id")),
    responses((status = 200, body = PresenceHeartbeatResponse), (status = 404, description = "Room not found"))
)]
pub async fn room_presence_heartbeat(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<PresenceHeartbeatResponse>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let online_count = state.presence.heartbeat(room_id, user.user_id).await;
    Ok(Json(PresenceHeartbeatResponse { online_count }))
}

/// Authenticated like (light per-user cooldown).
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/likes",
    tag = "rooms",
    params(("id" = String, Path, description = "Room id")),
    request_body = LikeRoomBody,
    responses((status = 200, body = LikeRoomResponse), (status = 404, description = "Room not found"))
)]
pub async fn room_like(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(_body): Json<LikeRoomBody>,
) -> Result<Json<LikeRoomResponse>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let (accepted, like_count) = state.presence.like(room_id, user.user_id).await;
    Ok(Json(LikeRoomResponse {
        accepted,
        like_count,
    }))
}
