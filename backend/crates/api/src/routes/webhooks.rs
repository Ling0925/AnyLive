//! SRS HTTP hooks: on_publish / on_unpublish.

use std::sync::Arc;

use anylive_domain::{RoomId, RoomStatus};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

/// SRS on_publish callback payload (subset).
#[derive(Debug, Deserialize, ToSchema)]
pub struct SrsPublishHook {
    /// Stream name — we use room UUID.
    #[serde(default)]
    pub stream: String,
    #[serde(default)]
    pub app: String,
    /// SRS query string (kept for payload compatibility; unused in P1).
    #[serde(default)]
    #[allow(dead_code)]
    pub param: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SrsHookResponse {
    /// SRS expects code 0 to allow.
    pub code: i32,
}

fn parse_room_stream(stream: &str) -> Option<RoomId> {
    let name = stream.trim().trim_start_matches('/');
    // Accept raw uuid or "uuid.flv" style
    let base = name.split('.').next().unwrap_or(name);
    Uuid::parse_str(base).ok().map(RoomId)
}

/// POST /api/v1/webhooks/srs/on_publish
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/srs/on_publish",
    tag = "webhooks",
    request_body = SrsPublishHook,
    responses((status = 200, body = SrsHookResponse))
)]
pub async fn srs_on_publish(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SrsPublishHook>,
) -> Result<(StatusCode, Json<SrsHookResponse>), ApiError> {
    let Some(room_id) = parse_room_stream(&body.stream) else {
        return Ok((StatusCode::OK, Json(SrsHookResponse { code: 1 })));
    };
    match state.rooms.get(room_id).await {
        Ok(room) if room.status == RoomStatus::Live => {
            tracing::info!(room = %room_id.0, app = %body.app, "srs on_publish allowed");
            Ok((StatusCode::OK, Json(SrsHookResponse { code: 0 })))
        }
        Ok(_) => {
            tracing::warn!(room = %room_id.0, "srs on_publish denied: not live");
            Ok((StatusCode::OK, Json(SrsHookResponse { code: 1 })))
        }
        Err(_) => {
            tracing::warn!(stream = %body.stream, "srs on_publish denied: unknown room");
            Ok((StatusCode::OK, Json(SrsHookResponse { code: 1 })))
        }
    }
}

/// POST /api/v1/webhooks/srs/on_unpublish
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/srs/on_unpublish",
    tag = "webhooks",
    request_body = SrsPublishHook,
    responses((status = 200, body = SrsHookResponse))
)]
pub async fn srs_on_unpublish(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SrsPublishHook>,
) -> Result<(StatusCode, Json<SrsHookResponse>), ApiError> {
    if let Some(room_id) = parse_room_stream(&body.stream) {
        if let Ok(room) = state.rooms.get(room_id).await {
            if room.status == RoomStatus::Live {
                // Best-effort auto-stop when encoder disconnects.
                let _ = state.rooms.stop(room_id, room.owner_id).await;
                tracing::info!(room = %room_id.0, "srs on_unpublish -> room stopped");
            }
        }
    }
    Ok((StatusCode::OK, Json(SrsHookResponse { code: 0 })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stream_uuid() {
        let id = Uuid::new_v4();
        let room = parse_room_stream(&id.to_string()).unwrap();
        assert_eq!(room.0, id);
        let room2 = parse_room_stream(&format!("{id}.flv")).unwrap();
        assert_eq!(room2.0, id);
        assert!(parse_room_stream("not-a-uuid").is_none());
    }
}
