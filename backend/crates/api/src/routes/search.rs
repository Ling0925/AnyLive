//! Simple user / room search (WBS E6.3).

use std::sync::Arc;

use anylive_common::AppError;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::routes::auth::UserDto;
use crate::routes::rooms::RoomDto;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Substring match (case-insensitive) against display name / room title.
    pub q: String,
    /// `users` | `rooms` | `all` (default all).
    #[serde(default)]
    pub r#type: Option<String>,
    /// Max items per category (1..=50, default 20).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    pub users: Vec<UserDto>,
    pub rooms: Vec<RoomDto>,
}

/// Placeholder schema for OpenAPI (query params documented on path).
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchQuerySchema {
    pub q: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub limit: Option<u32>,
}

/// Search users by display name and rooms by title.
#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "social",
    params(
        ("q" = String, Query, description = "Search query"),
        ("type" = Option<String>, Query, description = "users|rooms|all"),
        ("limit" = Option<u32>, Query, description = "1..=50 per category")
    ),
    responses((status = 200, body = SearchResponse))
)]
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    let needle = q.q.trim();
    if needle.is_empty() {
        return Err(ApiError(AppError::validation("q must not be empty")));
    }
    if needle.len() > 64 {
        return Err(ApiError(AppError::validation("q too long (max 64)")));
    }
    let limit = q.limit.unwrap_or(20).clamp(1, 50) as usize;
    let kind = q
        .r#type
        .as_deref()
        .unwrap_or("all")
        .trim()
        .to_ascii_lowercase();

    let want_users = matches!(kind.as_str(), "all" | "users" | "user");
    let want_rooms = matches!(kind.as_str(), "all" | "rooms" | "room");
    if !want_users && !want_rooms {
        return Err(ApiError(AppError::validation(
            "type must be users, rooms, or all",
        )));
    }

    let users = if want_users {
        state
            .auth
            .users()
            .search_display_name(needle, limit)
            .await
            .map_err(ApiError::from)?
            .into_iter()
            .map(UserDto::from)
            .collect()
    } else {
        Vec::new()
    };

    let rooms = if want_rooms {
        state
            .rooms
            .search_title(needle, limit)
            .await
            .into_iter()
            .map(RoomDto::from)
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(SearchResponse { users, rooms }))
}
