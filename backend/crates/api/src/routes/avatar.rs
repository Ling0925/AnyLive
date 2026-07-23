//! Avatar upload control plane (WBS E2.3).

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::object_storage::{
    presign_avatar_put, resolve_confirm_url, validate_blob_token, AvatarConfirmBody,
    AvatarPresignBody, AvatarPresignResponse,
};
use crate::routes::auth::UserDto;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AvatarBlobQuery {
    pub object_key: String,
    pub expires: u64,
    pub token: String,
}

/// Issue a short-lived avatar upload URL (MinIO when enabled, else API blob).
#[utoipa::path(
    post,
    path = "/api/v1/me/avatar/presign",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = AvatarPresignBody,
    responses((status = 200, body = AvatarPresignResponse))
)]
pub async fn avatar_presign(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<AvatarPresignBody>,
) -> Result<Json<AvatarPresignResponse>, ApiError> {
    let ct = body
        .content_type
        .as_deref()
        .unwrap_or("image/jpeg")
        .to_string();
    if !(ct.starts_with("image/") || ct == "application/octet-stream") {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "content_type must be image/*",
        )));
    }
    let resp = presign_avatar_put(&state.object_storage, user.user_id, &ct)
        .map_err(ApiError::from)?;
    Ok(Json(resp))
}

/// Confirm avatar URL after client PUT (stores on profile_extras).
#[utoipa::path(
    post,
    path = "/api/v1/me/avatar/confirm",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = AvatarConfirmBody,
    responses((status = 200, body = UserDto))
)]
pub async fn avatar_confirm(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<AvatarConfirmBody>,
) -> Result<Json<UserDto>, ApiError> {
    let url = resolve_confirm_url(
        &state.object_storage,
        user.user_id,
        &body.object_key,
        body.public_url,
    )
    .map_err(ApiError::from)?;
    let extras = state
        .profile_extras
        .set_avatar_url(user.user_id, Some(url))
        .await;
    let u = state.auth.me(user.user_id).await.map_err(ApiError::from)?;
    Ok(Json(UserDto::from_user(
        u,
        extras.age_confirmed(),
        extras.privacy_accepted(),
        extras.avatar_url.clone(),
        extras.region.clone(),
    )))
}

/// Synthetic offline blob sink (when MinIO is not enabled). Accepts body, validates token.
#[utoipa::path(
    put,
    path = "/api/v1/me/avatar/blob",
    tag = "users",
    security(("bearerAuth" = [])),
    responses((status = 204, description = "Blob accepted"))
)]
pub async fn avatar_blob_put(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(q): Query<AvatarBlobQuery>,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    if body.len() > 5 * 1024 * 1024 {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "avatar too large (max 5 MiB)",
        )));
    }
    validate_blob_token(
        &state.object_storage,
        user.user_id,
        &q.object_key,
        q.expires,
        &q.token,
    )
    .map_err(ApiError::from)?;
    // Bytes intentionally discarded in control-plane mode; public_url is virtual.
    let _ = body;
    Ok(StatusCode::NO_CONTENT)
}
