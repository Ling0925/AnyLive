//! Co-host invite + PK control-plane routes (P3).

use std::sync::Arc;

use anylive_domain::{InteractiveSession, PkSession, RoomId, RoomStatus, UserId};
use anylive_realtime::MessageEnvelope;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

fn session_dto(s: &InteractiveSession) -> InteractiveSessionDto {
    InteractiveSessionDto {
        id: s.id.to_string(),
        room_id: s.room_id.0.to_string(),
        host_id: s.host_id.0.to_string(),
        invitee_id: s.invitee_id.0.to_string(),
        status: s.status.as_str().to_string(),
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
        ended_at: s.ended_at.map(|t| t.to_rfc3339()),
    }
}

fn pk_dto(p: &PkSession) -> PkSessionDto {
    PkSessionDto {
        id: p.id.to_string(),
        room_a_id: p.room_a_id.0.to_string(),
        room_b_id: p.room_b_id.0.to_string(),
        host_a_id: p.host_a_id.0.to_string(),
        host_b_id: p.host_b_id.0.to_string(),
        status: p.status.as_str().to_string(),
        score_a: p.score_a,
        score_b: p.score_b,
        winner_room_id: p.winner_room_id.map(|r| r.0.to_string()),
        started_at: p.started_at.to_rfc3339(),
        ends_at: p.ends_at.to_rfc3339(),
        ended_at: p.ended_at.map(|t| t.to_rfc3339()),
        updated_at: p.updated_at.to_rfc3339(),
    }
}

async fn publish_room(state: &AppState, room_id: RoomId, event_type: &str, payload: serde_json::Value) {
    let channel = MessageEnvelope::room_channel(room_id);
    let data = serde_json::json!({
        "type": event_type,
        "payload": payload,
    });
    if let Err(e) = state.centrifugo_publisher.publish(&channel, data).await {
        tracing::warn!(error = %e, %channel, "centrifugo interactive publish failed");
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InteractiveInviteBody {
    pub invitee_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InteractiveRespondBody {
    pub accept: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InteractiveSessionDto {
    pub id: String,
    pub room_id: String,
    pub host_id: String,
    pub invitee_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InteractiveSessionListResponse {
    pub items: Vec<InteractiveSessionDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StartPkBody {
    pub opponent_room_id: String,
    #[serde(default)]
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PkSessionDto {
    pub id: String,
    pub room_a_id: String,
    pub room_b_id: String,
    pub host_a_id: String,
    pub host_b_id: String,
    pub status: String,
    pub score_a: i64,
    pub score_b: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_room_id: Option<String>,
    pub started_at: String,
    pub ends_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PkSessionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<PkSessionDto>,
}

/// Host invites a viewer/user to co-host.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/interactive/invite",
    tag = "rooms",
    security(("bearerAuth" = [])),
    responses((status = 201, body = InteractiveSessionDto))
)]
pub async fn interactive_invite(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<InteractiveInviteBody>,
) -> Result<(StatusCode, Json<InteractiveSessionDto>), ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let room = state.rooms.get(room_id).await.map_err(ApiError::from)?;
    if room.owner_id != user.user_id {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "only room owner may invite co-host",
        )));
    }
    state.features.require_cohost().map_err(ApiError::from)?;
    let invitee_uuid = Uuid::parse_str(&body.invitee_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid invitee_id")))?;
    let invitee = UserId(invitee_uuid);
    let session = state
        .interactive
        .invite(room_id, user.user_id, invitee)
        .await
        .map_err(ApiError::from)?;
    let dto = session_dto(&session);
    publish_room(
        &state,
        room_id,
        "interactive.invite",
        serde_json::to_value(&dto).unwrap_or_default(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(dto)))
}

/// Invitee accepts or declines.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/interactive/respond",
    tag = "rooms",
    security(("bearerAuth" = [])),
    responses((status = 200, body = InteractiveSessionDto))
)]
pub async fn interactive_respond(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<InteractiveRespondBody>,
) -> Result<Json<InteractiveSessionDto>, ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let session = state
        .interactive
        .respond(room_id, user.user_id, body.accept)
        .await
        .map_err(ApiError::from)?;
    let dto = session_dto(&session);
    publish_room(
        &state,
        room_id,
        if body.accept {
            "interactive.accepted"
        } else {
            "interactive.declined"
        },
        serde_json::to_value(&dto).unwrap_or_default(),
    )
    .await;
    Ok(Json(dto))
}

/// Host or co-host leaves / ends session.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/interactive/leave",
    tag = "rooms",
    security(("bearerAuth" = [])),
    responses((status = 200, body = InteractiveSessionDto))
)]
pub async fn interactive_leave(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<InteractiveSessionDto>, ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let session = state
        .interactive
        .leave(room_id, user.user_id)
        .await
        .map_err(ApiError::from)?;
    let dto = session_dto(&session);
    publish_room(
        &state,
        room_id,
        "interactive.ended",
        serde_json::to_value(&dto).unwrap_or_default(),
    )
    .await;
    Ok(Json(dto))
}

/// List interactive sessions for a room.
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{id}/interactive",
    tag = "rooms",
    responses((status = 200, body = InteractiveSessionListResponse))
)]
pub async fn list_interactive(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<InteractiveSessionListResponse>, ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let items = state
        .interactive
        .list_for_room(room_id)
        .await
        .iter()
        .map(session_dto)
        .collect();
    Ok(Json(InteractiveSessionListResponse { items }))
}

/// Current PK for room (auto-expires on read).
#[utoipa::path(
    get,
    path = "/api/v1/rooms/{id}/pk",
    tag = "rooms",
    responses((status = 200, body = PkSessionResponse))
)]
pub async fn get_pk(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PkSessionResponse>, ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let session = state
        .interactive
        .get_pk_for_room(room_id)
        .await
        .map(|p| pk_dto(&p));
    Ok(Json(PkSessionResponse { session }))
}

/// Start timed PK against another live room.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/pk/start",
    tag = "rooms",
    security(("bearerAuth" = [])),
    responses((status = 201, body = PkSessionDto))
)]
pub async fn start_pk(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<StartPkBody>,
) -> Result<(StatusCode, Json<PkSessionDto>), ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_a = RoomId(room_uuid);
    let room = state.rooms.get(room_a).await.map_err(ApiError::from)?;
    if room.owner_id != user.user_id {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "only room owner may start PK",
        )));
    }
    state.features.require_pk().map_err(ApiError::from)?;
    if room.status != RoomStatus::Live {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::RoomNotLive,
            "room is not live",
        )));
    }
    let opp_uuid = Uuid::parse_str(&body.opponent_room_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid opponent_room_id")))?;
    let room_b = RoomId(opp_uuid);
    let opp = state.rooms.get(room_b).await.map_err(ApiError::from)?;
    if opp.status != RoomStatus::Live {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Conflict,
            "opponent room is not live",
        )));
    }
    let pk = state
        .interactive
        .start_pk(
            room_a,
            room.owner_id,
            room_b,
            opp.owner_id,
            body.duration_secs,
        )
        .await
        .map_err(ApiError::from)?;
    let dto = pk_dto(&pk);
    let payload = serde_json::to_value(&dto).unwrap_or_default();
    publish_room(&state, room_a, "pk.started", payload.clone()).await;
    publish_room(&state, room_b, "pk.started", payload).await;
    Ok((StatusCode::CREATED, Json(dto)))
}

/// End active PK early.
#[utoipa::path(
    post,
    path = "/api/v1/rooms/{id}/pk/end",
    tag = "rooms",
    security(("bearerAuth" = [])),
    responses((status = 200, body = PkSessionDto))
)]
pub async fn end_pk(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<PkSessionDto>, ApiError> {
    let room_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room_id = RoomId(room_uuid);
    let pk = state
        .interactive
        .end_pk(room_id, user.user_id)
        .await
        .map_err(ApiError::from)?;
    let dto = pk_dto(&pk);
    let payload = serde_json::to_value(&dto).unwrap_or_default();
    publish_room(&state, pk.room_a_id, "pk.ended", payload.clone()).await;
    publish_room(&state, pk.room_b_id, "pk.ended", payload).await;
    Ok(Json(dto))
}
