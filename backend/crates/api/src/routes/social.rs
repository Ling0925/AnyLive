//! Social HTTP handlers: follow / unfollow.

use std::sync::Arc;

use anylive_domain::UserId;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct FollowingListResponse {
    pub user_ids: Vec<String>,
}

/// POST /api/v1/users/{id}/follow
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/follow",
    tag = "social",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "User UUID to follow")),
    responses((status = 204, description = "Following"))
)]
pub async fn follow_user(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let followee = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid user id")))?;
    state
        .social
        .follow(user.user_id, UserId(followee))
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/{id}/follow
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/follow",
    tag = "social",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "User UUID to unfollow")),
    responses((status = 204, description = "Unfollowed"))
)]
pub async fn unfollow_user(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let followee = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid user id")))?;
    state
        .social
        .unfollow(user.user_id, UserId(followee))
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/me/following
#[utoipa::path(
    get,
    path = "/api/v1/me/following",
    tag = "social",
    security(("bearerAuth" = [])),
    responses((status = 200, body = FollowingListResponse))
)]
pub async fn list_following(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<FollowingListResponse>, ApiError> {
    let ids = state.social.following_ids(user.user_id).await;
    Ok(Json(FollowingListResponse {
        user_ids: ids.into_iter().map(|u| u.0.to_string()).collect(),
    }))
}
