//! Following live feed + light hot ranking (P4 v1).

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use chrono::Utc;
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

/// GET /api/v1/feed/hot — live rooms ranked by follower count then recency (P4 light recommend v1).
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
    let mut scored: Vec<(i64, _)> = Vec::with_capacity(live.len());
    let now = Utc::now();
    for room in live {
        let followers = state.social.follower_count(room.owner_id).await as i64;
        // Recency bonus: rooms updated in last hour get up to +50, decaying linearly.
        let age_secs = (now - room.updated_at).num_seconds().max(0);
        let recency_bonus = if age_secs >= 3600 {
            0
        } else {
            50 - (age_secs * 50 / 3600)
        };
        // Score = followers * 10 + recency; secondary sort by updated_at.
        let score = followers.saturating_mul(10).saturating_add(recency_bonus);
        scored.push((score, room));
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.updated_at.cmp(&a.1.updated_at))
    });
    Ok(Json(FeedResponse {
        items: scored.into_iter().map(|(_, r)| RoomDto::from(r)).collect(),
    }))
}
