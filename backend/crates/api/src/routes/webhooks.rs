//! SRS HTTP hooks: on_publish / on_unpublish.
//!
//! When `SRS_WEBHOOK_SECRET` is set, requests must present the same value via
//! header `X-AnyLive-Webhook-Secret` **or** query `?secret=` (SRS http_hooks
//! cannot set custom headers, so query is required for real OBS→SRS→API).
//! Prefer header when the caller can set it. Production should always set the
//! secret; local dogfood may leave it empty (open hooks).
//!
//! Publish stream names must be signed HMAC tokens issued by media publish —
//! bare room UUIDs are rejected.

use std::sync::Arc;

use anylive_domain::RoomStatus;
use anylive_media::{parse_room_from_stream, SrsMediaProvider};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::guards::constant_time_eq;
use crate::state::AppState;

/// SRS on_publish callback payload (subset).
#[derive(Debug, Deserialize, ToSchema)]
pub struct SrsPublishHook {
    /// Stream name — signed publish key or (legacy unpublish) room UUID.
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

#[derive(Debug, Deserialize, Default)]
pub struct WebhookQuery {
    #[serde(default)]
    pub secret: Option<String>,
}

/// Validate optional shared secret for SRS callbacks.
///
/// When `SRS_WEBHOOK_SECRET` is unset/empty the hook is open (local/dev only).
/// Production startup requires a non-empty secret via
/// [`crate::guards::check_srs_webhook_for_production`].
///
/// Accepts either:
/// - header `X-AnyLive-Webhook-Secret` (preferred for non-SRS callers), or
/// - query `?secret=` (required for stock SRS `http_hooks`, which cannot set headers).
pub fn check_webhook_secret(headers: &HeaderMap, query: &WebhookQuery) -> Result<(), ApiError> {
    let expected = match std::env::var("SRS_WEBHOOK_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => return Ok(()), // open in local when unset
    };
    let header = headers
        .get("x-anylive-webhook-secret")
        .and_then(|v| v.to_str().ok());
    let provided = header.or(query.secret.as_deref());
    match provided {
        Some(got) if constant_time_eq(got, &expected) => Ok(()),
        _ => Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "invalid webhook secret",
        ))),
    }
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
    headers: HeaderMap,
    Query(query): Query<WebhookQuery>,
    Json(body): Json<SrsPublishHook>,
) -> Result<(StatusCode, Json<SrsHookResponse>), ApiError> {
    check_webhook_secret(&headers, &query)?;
    // Strict signed stream validation: stream name = room UUID, auth in param or stream query.
    let room_id = match state
        .media
        .validate_publish_stream_with_param(&body.stream, &body.param)
    {
        Ok(id) => id,
        Err(e) => {
            // Also accept full OBS key in `stream` field alone.
            match state.media.validate_publish_stream(&body.stream) {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!(stream = %body.stream, param = %body.param, error = %e, "srs on_publish denied: bad stream key");
                    return Ok((StatusCode::OK, Json(SrsHookResponse { code: 1 })));
                }
            }
        }
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
    headers: HeaderMap,
    Query(query): Query<WebhookQuery>,
    Json(body): Json<SrsPublishHook>,
) -> Result<(StatusCode, Json<SrsHookResponse>), ApiError> {
    check_webhook_secret(&headers, &query)?;
    // Unpublish accepts signed keys (stream+param or combined) or bare room id.
    let room_id = state
        .media
        .validate_publish_stream_with_param(&body.stream, &body.param)
        .ok()
        .or_else(|| {
            state
                .media
                .validate_publish_stream(&body.stream)
                .ok()
        })
        .or_else(|| parse_room_from_stream(&body.stream));
    if let Some(room_id) = room_id {
        if let Ok(room) = state.rooms.get(room_id).await {
            if room.status == RoomStatus::Live {
                // Best-effort auto-stop when encoder disconnects.
                let _ = state.rooms.stop(room_id, room.owner_id).await;
                tracing::info!(room = %room_id.0, "srs on_unpublish -> room stopped");
            }
        }
        state.media.clear_active_stream(room_id).await;
    }
    Ok((StatusCode::OK, Json(SrsHookResponse { code: 0 })))
}

// Re-export for tests that previously used free parse helper.
#[allow(dead_code)]
fn _ensure_provider_type(_: &SrsMediaProvider) {}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::sync::Mutex;

    /// Env vars are process-global; serialize webhook secret tests to avoid races.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn parse_stream_uuid() {
        let id = uuid::Uuid::new_v4();
        let room = parse_room_from_stream(&id.to_string()).unwrap();
        assert_eq!(room.0, id);
        let room2 = parse_room_from_stream(&format!("{id}.flv")).unwrap();
        assert_eq!(room2.0, id);
        assert!(parse_room_from_stream("not-a-uuid").is_none());
    }

    #[test]
    fn webhook_secret_open_when_unset() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Other tests may set this env var; force open mode for this case.
        std::env::remove_var("SRS_WEBHOOK_SECRET");
        // Also tolerate empty string as open.
        std::env::set_var("SRS_WEBHOOK_SECRET", "");
        let headers = HeaderMap::new();
        let q = WebhookQuery::default();
        let res = check_webhook_secret(&headers, &q);
        std::env::remove_var("SRS_WEBHOOK_SECRET");
        assert!(res.is_ok(), "empty secret should be open");
    }

    #[test]
    fn webhook_secret_rejects_missing() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SRS_WEBHOOK_SECRET", "s3cret");
        let headers = HeaderMap::new();
        let q = WebhookQuery::default();
        assert!(check_webhook_secret(&headers, &q).is_err());
        std::env::remove_var("SRS_WEBHOOK_SECRET");
    }

    #[test]
    fn webhook_secret_accepts_header() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SRS_WEBHOOK_SECRET", "s3cret");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-anylive-webhook-secret",
            HeaderValue::from_static("s3cret"),
        );
        let q = WebhookQuery::default();
        assert!(check_webhook_secret(&headers, &q).is_ok());
        std::env::remove_var("SRS_WEBHOOK_SECRET");
    }

    #[test]
    fn webhook_secret_accepts_query() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SRS_WEBHOOK_SECRET", "s3cret");
        let headers = HeaderMap::new();
        let q = WebhookQuery {
            secret: Some("s3cret".into()),
        };
        assert!(check_webhook_secret(&headers, &q).is_ok());
        std::env::remove_var("SRS_WEBHOOK_SECRET");
    }

    #[test]
    fn webhook_secret_rejects_wrong_query() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("SRS_WEBHOOK_SECRET", "s3cret");
        let headers = HeaderMap::new();
        let q = WebhookQuery {
            secret: Some("wrong".into()),
        };
        assert!(check_webhook_secret(&headers, &q).is_err());
        std::env::remove_var("SRS_WEBHOOK_SECRET");
    }
}
