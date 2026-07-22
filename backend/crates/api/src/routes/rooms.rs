//! Room and media control-plane HTTP handlers.

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{Room, RoomId, RoomStatus};
use anylive_media::{MediaProvider, PlayUrls, PublishInfo};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoomBody {
    pub title: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListRoomsQuery {
    /// Filter by status: idle | live | closed
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoomDto {
    pub id: String,
    pub owner_id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Room> for RoomDto {
    fn from(r: Room) -> Self {
        Self {
            id: r.id.0.to_string(),
            owner_id: r.owner_id.0.to_string(),
            title: r.title,
            status: r.status.as_str().to_string(),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoomListResponse {
    pub items: Vec<RoomDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublishInfoDto {
    pub push_url: String,
    pub stream_key: String,
    pub expires_at: String,
}

impl From<PublishInfo> for PublishInfoDto {
    fn from(p: PublishInfo) -> Self {
        Self {
            push_url: p.push_url,
            stream_key: p.stream_key,
            expires_at: p.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlayUrlsDto {
    pub hls: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flv: Option<String>,
}

impl From<PlayUrls> for PlayUrlsDto {
    fn from(p: PlayUrls) -> Self {
        Self {
            hls: p.hls,
            flv: p.flv,
        }
    }
}

fn parse_room_id(id: &str) -> Result<RoomId, ApiError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|_| ApiError(AppError::validation("invalid room id")))?;
    Ok(RoomId(uuid))
}

/// Create a room (auth required).
#[utoipa::path(
    post,
    path = "/api/v1/rooms",
    tag = "rooms",
    security(("bearerAuth" = [])),
    request_body = CreateRoomBody,
    responses((status = 201, description = "Created", body = RoomDto))
)]
pub async fn create_room(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<CreateRoomBody>,
) -> Result<(StatusCode, Json<RoomDto>), ApiError> {
    let room = state
        .rooms
        .create(user.user_id, body.title)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(room.into())))
}

/// List rooms, optionally filtered by status.
#[utoipa::path(
    get,
    path = "/api/v1/rooms",
    tag = "rooms",
    params(("status" = Option<String>, Query, description = "idle|live|closed")),
    responses((status = 200, body = RoomListResponse))
)]
pub async fn list_rooms(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListRoomsQuery>,
) -> Result<Json<RoomListResponse>, ApiError> {
    let status = match q.status.as_deref() {
        None => None,
        Some(s) => Some(
            RoomStatus::parse(s)
                .ok_or_else(|| ApiError(AppError::validation("invalid status filter")))?,
        ),
    };
    let items = state.rooms.list(status).await;
    Ok(Json(RoomListResponse {
        items: items.into_iter().map(RoomDto::from).collect(),
    }))
}

/// Get room by id.
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{id}",
    tag = "rooms",
    params(("id" = String, Path, description = "Room UUID")),
    responses((status = 200, body = RoomDto))
)]
pub async fn get_room(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RoomDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let room = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    Ok(Json(room.into()))
}

/// Start live (owner only). Idle -> Live.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/start",
    tag = "rooms",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Room UUID")),
    responses((status = 200, body = RoomDto))
)]
pub async fn start_room(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<RoomDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let room = state
        .rooms
        .start(room_id, user.user_id)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(room.into()))
}

/// Stop live (owner only). Live -> Idle.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/stop",
    tag = "rooms",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Room UUID")),
    responses((status = 200, body = RoomDto))
)]
pub async fn stop_room(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<RoomDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let room = state
        .rooms
        .stop(room_id, user.user_id)
        .await
        .map_err(ApiError::from)?;
    // Drop active signed stream mapping so play falls back after stop.
    state.media.clear_active_stream(room_id).await;
    Ok(Json(room.into()))
}

/// Issue RTMP publish credentials (owner only).
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/media/publish",
    tag = "rooms",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Room UUID")),
    responses((status = 200, body = PublishInfoDto))
)]
pub async fn media_publish(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<PublishInfoDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let room = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    if room.owner_id != user.user_id {
        return Err(ApiError(AppError::new(
            ErrorCode::Forbidden,
            "not room owner",
        )));
    }
    let info = state
        .media
        .issue_publish(room_id, user.user_id)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(info.into()))
}

/// Play URLs (public when room is live; otherwise ROOM_NOT_LIVE).
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{id}/media/play",
    tag = "rooms",
    params(("id" = String, Path, description = "Room UUID")),
    responses((status = 200, body = PlayUrlsDto))
)]
pub async fn media_play(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PlayUrlsDto>, ApiError> {
    let room_id = parse_room_id(&id)?;
    let room = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    if room.status != RoomStatus::Live {
        return Err(ApiError(AppError::new(
            ErrorCode::RoomNotLive,
            "room is not live",
        )));
    }
    let urls = state
        .media
        .play_urls(room_id)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(urls.into()))
}
