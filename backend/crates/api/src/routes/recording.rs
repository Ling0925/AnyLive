//! Room recording enable/disable HTTP handlers (WBS E3.5).

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
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
pub struct RecordingStatusDto {
    pub room_id: String,
    pub recording_enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetRecordingBody {
    pub enabled: bool,
}

/// Get recording flag for a room (public read).
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{id}/recording",
    tag = "rooms",
    params(("id" = String, Path, description = "Room id")),
    responses((status = 200, body = RecordingStatusDto), (status = 404, description = "Room not found"))
)]
pub async fn get_recording(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RecordingStatusDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let _ = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    let enabled = state.recording.get(room_id).await;
    Ok(Json(RecordingStatusDto {
        room_id: room_id.0.to_string(),
        recording_enabled: enabled,
    }))
}

/// Owner-only recording toggle (control plane; no media egress).
#[utoipa::path(
    put,
    path = "/api/v1/rooms/{id}/recording",
    tag = "rooms",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Room id")),
    request_body = SetRecordingBody,
    responses((status = 200, body = RecordingStatusDto), (status = 403, description = "Not owner"))
)]
pub async fn set_recording(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<SetRecordingBody>,
) -> Result<Json<RecordingStatusDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let room = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    if room.owner_id != user.user_id {
        return Err(ApiError(AppError::new(
            ErrorCode::Forbidden,
            "not room owner",
        )));
    }
    let enabled = state.recording.set(room_id, body.enabled).await;
    Ok(Json(RecordingStatusDto {
        room_id: room_id.0.to_string(),
        recording_enabled: enabled,
    }))
}
