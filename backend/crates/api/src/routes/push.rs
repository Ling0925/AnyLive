//! Push device token registration routes (WBS E8.9 scaffold).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

const PLATFORMS: &[&str] = &["ios", "android", "web", "other"];

#[derive(Debug, Deserialize, ToSchema)]
pub struct PushRegisterBody {
    /// Device push token from FCM/APNs/WebPush (opaque string).
    pub token: String,
    /// ios | android | web | other
    pub platform: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PushDeviceDto {
    pub id: String,
    pub platform: String,
    pub token: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PushDeviceListResponse {
    pub items: Vec<PushDeviceDto>,
}

fn normalize_platform(raw: &str) -> Result<String, ApiError> {
    let p = raw.trim().to_ascii_lowercase();
    if PLATFORMS.contains(&p.as_str()) {
        Ok(p)
    } else {
        Err(ApiError::from(anylive_common::AppError::validation(
            "platform must be ios, android, web, or other",
        )))
    }
}

fn validate_token(token: &str) -> Result<&str, ApiError> {
    let t = token.trim();
    if t.is_empty() {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "token must not be empty",
        )));
    }
    if t.len() > 512 {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "token too long (max 512)",
        )));
    }
    Ok(t)
}

/// Register or upsert a device push token for the current user.
#[utoipa::path(
    post,
    path = "/api/v1/me/push-tokens",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = PushRegisterBody,
    responses((status = 200, body = PushDeviceDto))
)]
pub async fn register_push_token(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<PushRegisterBody>,
) -> Result<Json<PushDeviceDto>, ApiError> {
    let token = validate_token(&body.token)?;
    let platform = normalize_platform(&body.platform)?;
    let device = state
        .push
        .register(user.user_id, platform, token.to_string())
        .await;
    Ok(Json(PushDeviceDto {
        id: device.id.to_string(),
        platform: device.platform,
        token: device.token,
        created_at: device.created_at.to_rfc3339(),
        updated_at: device.updated_at.to_rfc3339(),
    }))
}

/// List push tokens registered for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/me/push-tokens",
    tag = "users",
    security(("bearerAuth" = [])),
    responses((status = 200, body = PushDeviceListResponse))
)]
pub async fn list_push_tokens(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<PushDeviceListResponse>, ApiError> {
    let items = state
        .push
        .list_for_user(user.user_id)
        .await
        .into_iter()
        .map(|d| PushDeviceDto {
            id: d.id.to_string(),
            platform: d.platform,
            token: d.token,
            created_at: d.created_at.to_rfc3339(),
            updated_at: d.updated_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(PushDeviceListResponse { items }))
}

/// Unregister a device push token (body carries token string).
#[utoipa::path(
    delete,
    path = "/api/v1/me/push-tokens",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = PushRegisterBody,
    responses(
        (status = 204, description = "Unregistered"),
        (status = 404, description = "Token not found for user")
    )
)]
pub async fn unregister_push_token(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<PushRegisterBody>,
) -> Result<StatusCode, ApiError> {
    let token = validate_token(&body.token)?;
    // platform ignored on delete; still validate if present
    let _ = normalize_platform(if body.platform.trim().is_empty() {
        "other"
    } else {
        &body.platform
    })?;
    let removed = state.push.unregister(user.user_id, token).await;
    if !removed {
        return Err(ApiError::from(anylive_common::AppError::not_found(
            "push token not found",
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PushTestBody {
    pub title: String,
    pub body: String,
    /// Optional specific device token; when omitted, sends to all of the user.
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PushTestResponse {
    pub delivery: String,
    pub attempted: usize,
    pub succeeded: usize,
}

/// Fire a test push through the configured delivery backend (WBS E8.9).
/// No-ops when `PUSH_DELIVERY=noop` (default).
#[utoipa::path(
    post,
    path = "/api/v1/me/push-tokens/test",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = PushTestBody,
    responses((status = 200, body = PushTestResponse))
)]
pub async fn test_push(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<PushTestBody>,
) -> Result<Json<PushTestResponse>, ApiError> {
    let title = body.title.trim();
    let text = body.body.trim();
    if title.is_empty() || text.is_empty() {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "title and body are required",
        )));
    }
    let devices = state.push.list_for_user(user.user_id).await;
    let targets: Vec<_> = if let Some(tok) = body.token.as_deref() {
        let t = validate_token(tok)?;
        devices.into_iter().filter(|d| d.token == t).collect()
    } else {
        devices
    };
    if targets.is_empty() {
        return Err(ApiError::from(anylive_common::AppError::not_found(
            "no matching push token",
        )));
    }
    let msg = crate::push_delivery::PushMessage {
        title: title.to_string(),
        body: text.to_string(),
        data: std::collections::HashMap::from([("kind".into(), "test".into())]),
    };
    let mut succeeded = 0usize;
    for d in &targets {
        if state
            .push_delivery
            .send(&d.token, &d.platform, &msg)
            .await
            .is_ok()
        {
            succeeded += 1;
        }
    }
    Ok(Json(PushTestResponse {
        delivery: state.push_delivery.kind().to_string(),
        attempted: targets.len(),
        succeeded,
    }))
}
