//! Following live feed: rooms owned by users the caller follows.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::routes::rooms::RoomDto;
use crate::state::AppState;
use anylive_domain::RoomStatus;

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedResponse {
    pub items: Vec<RoomDto>,
}

/// GET /api/v1/feed/following — live rooms from followed hosts.
#[utoipa::path(
    get,
    path = "/api/v1/feed/following",
    tag = "social",
    security(("bearerAuth" = [])),
    responses((status = 200, body = FeedResponse))
)]
pub async fn feed_following(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<FeedResponse>, ApiError> {
    let following = state.social.following_ids(user.user_id).await;
    let live = state.rooms.list(Some(RoomStatus::Live)).await;
    let items = live
        .into_iter()
        .filter(|r| following.iter().any(|f| f.0 == r.owner_id.0))
        .map(RoomDto::from)
        .collect();
    Ok(Json(FeedResponse { items }))
}

/// GET /api/v1/feed/hot — currently live rooms (simple hot = all live).
#[utoipa::path(
    get,
    path = "/api/v1/feed/hot",
    tag = "social",
    responses((status = 200, body = FeedResponse))
)]
pub async fn feed_hot(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FeedResponse>, ApiError> {
    let live = state.rooms.list(Some(RoomStatus::Live)).await;
    Ok(Json(FeedResponse {
        items: live.into_iter().map(RoomDto::from).collect(),
    }))
}
