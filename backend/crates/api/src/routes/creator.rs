//! Creator center stats (P4 scaffold).

use std::sync::Arc;

use anylive_domain::RoomStatus;
use anylive_wallet::LedgerType;
use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::routes::rooms::RoomDto;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatorStatsResponse {
    pub follower_count: i64,
    pub following_count: i64,
    pub live_rooms: i64,
    pub total_rooms: i64,
    pub gift_coins_received: i64,
    pub gift_credit_entries: i64,
    pub rooms: Vec<RoomDto>,
}

/// GET /api/v1/me/creator — host dashboard: followers, rooms, gift income.
#[utoipa::path(
    get,
    path = "/api/v1/me/creator",
    tag = "users",
    security(("bearerAuth" = [])),
    responses((status = 200, body = CreatorStatsResponse))
)]
pub async fn creator_stats(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<CreatorStatsResponse>, ApiError> {
    let follower_count = state.social.follower_count(user.user_id).await as i64;
    let following_count = state.social.following_count(user.user_id).await as i64;
    let rooms = state.rooms.list_by_owner(user.user_id, None).await;
    let live_rooms = rooms
        .iter()
        .filter(|r| r.status == RoomStatus::Live)
        .count() as i64;
    let total_rooms = rooms.len() as i64;
    let ledger = state.wallet.ledger_for(user.user_id).await;
    let mut gift_coins_received = 0i64;
    let mut gift_credit_entries = 0i64;
    for e in ledger {
        if e.entry_type == LedgerType::GiftCredit {
            gift_coins_received = gift_coins_received.saturating_add(e.amount.max(0));
            gift_credit_entries += 1;
        }
    }
    Ok(Json(CreatorStatsResponse {
        follower_count,
        following_count,
        live_rooms,
        total_rooms,
        gift_coins_received,
        gift_credit_entries,
        rooms: rooms.into_iter().map(RoomDto::from).collect(),
    }))
}
