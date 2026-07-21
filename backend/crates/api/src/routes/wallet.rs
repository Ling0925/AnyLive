//! Wallet and gift HTTP handlers.

use std::sync::Arc;

use anylive_domain::UserId;
use anylive_wallet::{GiftCatalogItem, GiftOrder, WalletSnapshot};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletDto {
    pub balance: i64,
}

impl From<WalletSnapshot> for WalletDto {
    fn from(w: WalletSnapshot) -> Self {
        Self {
            balance: w.balance,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TopupBody {
    pub amount: i64,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GiftDto {
    pub id: String,
    pub name: String,
    pub price: i64,
}

impl From<GiftCatalogItem> for GiftDto {
    fn from(g: GiftCatalogItem) -> Self {
        Self {
            id: g.id.to_string(),
            name: g.name,
            price: g.price,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GiftListResponse {
    pub items: Vec<GiftDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendGiftBody {
    pub gift_id: String,
    pub receiver_id: String,
    #[serde(default = "default_count")]
    pub count: u32,
    pub client_request_id: String,
}

fn default_count() -> u32 {
    1
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GiftOrderDto {
    pub id: String,
    pub room_id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub gift_id: String,
    pub count: u32,
    pub total_coins: i64,
    pub client_request_id: String,
    pub replayed: bool,
}

impl GiftOrderDto {
    fn from_order(o: GiftOrder, replayed: bool) -> Self {
        Self {
            id: o.id.to_string(),
            room_id: o.room_id.to_string(),
            sender_id: o.sender_id.0.to_string(),
            receiver_id: o.receiver_id.0.to_string(),
            gift_id: o.gift_id.to_string(),
            count: o.count,
            total_coins: o.total_coins,
            client_request_id: o.client_request_id,
            replayed,
        }
    }
}

/// GET /api/v1/wallet
#[utoipa::path(get, path = "/api/v1/wallet", tag = "wallet", security(("bearerAuth" = [])), responses((status = 200, body = WalletDto)))]
pub async fn get_wallet(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<WalletDto>, ApiError> {
    let bal = state.wallet.balance(user.user_id).await;
    Ok(Json(WalletDto { balance: bal }))
}

/// POST /api/v1/wallet/topups — mock topup for P1 sandbox.
#[utoipa::path(post, path = "/api/v1/wallet/topups", tag = "wallet", security(("bearerAuth" = [])), request_body = TopupBody, responses((status = 200, body = WalletDto)))]
pub async fn topup_wallet(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<TopupBody>,
) -> Result<Json<WalletDto>, ApiError> {
    let snap = state
        .wallet
        .credit_topup(
            user.user_id,
            body.amount,
            body.reference.unwrap_or_else(|| "mock-topup".into()),
        )
        .await
        .map_err(ApiError::from)?;
    Ok(Json(snap.into()))
}

/// GET /api/v1/gifts
#[utoipa::path(get, path = "/api/v1/gifts", tag = "gifts", responses((status = 200, body = GiftListResponse)))]
pub async fn list_gifts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GiftListResponse>, ApiError> {
    let items = state.wallet.list_gifts().await;
    Ok(Json(GiftListResponse {
        items: items.into_iter().map(GiftDto::from).collect(),
    }))
}

/// POST /api/v1/rooms/{id}/gifts
#[utoipa::path(post, path = "/api/v1/rooms/{id}/gifts", tag = "gifts", security(("bearerAuth" = [])), request_body = SendGiftBody, responses((status = 200, body = GiftOrderDto)))]
pub async fn send_gift(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<SendGiftBody>,
) -> Result<(StatusCode, Json<GiftOrderDto>), ApiError> {
    let room_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    // Ensure room exists (and optionally live — allow idle for dogfood simplicity? Prefer exists)
    let _room = state
        .rooms
        .get(anylive_domain::RoomId(room_id))
        .await
        .map_err(ApiError::from)?;

    let gift_id = Uuid::parse_str(&body.gift_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid gift_id")))?;
    let receiver_uuid = Uuid::parse_str(&body.receiver_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid receiver_id")))?;
    let receiver = UserId(receiver_uuid);

    // Default receiver to room owner if client sends owner's id — already required.
    let (order, replayed) = state
        .wallet
        .send_gift(
            room_id,
            user.user_id,
            receiver,
            gift_id,
            body.count,
            body.client_request_id,
        )
        .await
        .map_err(ApiError::from)?;

    let status = if replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(GiftOrderDto::from_order(order, replayed))))
}
