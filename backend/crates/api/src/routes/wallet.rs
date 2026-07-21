//! Wallet and gift HTTP handlers.

use std::sync::Arc;

use anylive_domain::{RoomId, UserId};
use anylive_realtime::{gift_envelope, MessageEnvelope};
use anylive_wallet::{GiftCatalogItem, GiftOrder, LedgerEntry, WalletSnapshot};
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
    pub active: bool,
}

impl From<GiftCatalogItem> for GiftDto {
    fn from(g: GiftCatalogItem) -> Self {
        Self {
            id: g.id.to_string(),
            name: g.name,
            price: g.price,
            active: g.active,
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

#[derive(Debug, Serialize, ToSchema)]
pub struct LedgerEntryDto {
    pub id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub entry_type: String,
    pub reference: String,
    pub created_at: String,
}

impl From<LedgerEntry> for LedgerEntryDto {
    fn from(e: LedgerEntry) -> Self {
        Self {
            id: e.id.to_string(),
            amount: e.amount,
            balance_after: e.balance_after,
            entry_type: match e.entry_type {
                anylive_wallet::LedgerType::Topup => "topup".into(),
                anylive_wallet::LedgerType::GiftDebit => "gift_debit".into(),
                anylive_wallet::LedgerType::GiftCredit => "gift_credit".into(),
                anylive_wallet::LedgerType::Adjustment => "adjustment".into(),
            },
            reference: e.reference,
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LedgerListResponse {
    pub items: Vec<LedgerEntryDto>,
}

/// GET /api/v1/wallet/ledger
#[utoipa::path(get, path = "/api/v1/wallet/ledger", tag = "wallet", security(("bearerAuth" = [])), responses((status = 200, body = LedgerListResponse)))]
pub async fn get_wallet_ledger(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<LedgerListResponse>, ApiError> {
    let items = state.wallet.ledger_for(user.user_id).await;
    Ok(Json(LedgerListResponse {
        items: items.into_iter().map(LedgerEntryDto::from).collect(),
    }))
}

/// Max mock topup per request (sandbox only).
pub const MAX_MOCK_TOPUP_AMOUNT: i64 = 100_000;

/// POST /api/v1/wallet/topups — mock topup for local/dogfood sandbox only.
///
/// Disabled when `APP_ENV` is production/prod. Amount capped at
/// [`MAX_MOCK_TOPUP_AMOUNT`]. Prefer a unique `reference` for idempotency once
/// the wallet layer enforces unique topup references.
#[utoipa::path(post, path = "/api/v1/wallet/topups", tag = "wallet", security(("bearerAuth" = [])), request_body = TopupBody, responses((status = 200, body = WalletDto)))]
pub async fn topup_wallet(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<TopupBody>,
) -> Result<Json<WalletDto>, ApiError> {
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into());
    if crate::guards::is_production_env(&app_env) {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::ForbiddenPolicy,
            "mock topup disabled in production",
        )));
    }
    if body.amount <= 0 {
        return Err(ApiError(anylive_common::AppError::validation(
            "topup amount must be positive",
        )));
    }
    if body.amount > MAX_MOCK_TOPUP_AMOUNT {
        return Err(ApiError(anylive_common::AppError::validation(format!(
            "topup amount exceeds max {MAX_MOCK_TOPUP_AMOUNT}"
        ))));
    }
    // Default reference is unique per request so accidental double-taps do not
    // share a single non-idempotent "mock-topup" key once uniqueness lands.
    let reference = body.reference.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
        format!("mock-topup-{}", Uuid::new_v4())
    });
    let snap = state
        .wallet
        .credit_topup(user.user_id, body.amount, reference)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(snap.into()))
}

/// GET /api/v1/gifts — public catalog (active gifts only).
#[utoipa::path(get, path = "/api/v1/gifts", tag = "gifts", responses((status = 200, body = GiftListResponse)))]
pub async fn list_gifts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GiftListResponse>, ApiError> {
    let items = state.wallet.list_gifts().await;
    Ok(Json(GiftListResponse {
        items: items
            .into_iter()
            .filter(|g| g.active)
            .map(GiftDto::from)
            .collect(),
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
    if state.moderation.is_muted(user.user_id).await {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "user is muted",
        )));
    }
    let room_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room id")))?;
    let room = state
        .rooms
        .get(anylive_domain::RoomId(room_id))
        .await
        .map_err(ApiError::from)?;
    if room.status != anylive_domain::RoomStatus::Live {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::RoomNotLive,
            "room is not live",
        )));
    }

    let gift_id = Uuid::parse_str(&body.gift_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid gift_id")))?;
    let receiver_uuid = Uuid::parse_str(&body.receiver_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid receiver_id")))?;
    let receiver = UserId(receiver_uuid);

    // Showroom P1: gifts only go to the room owner.
    if room.owner_id != receiver {
        return Err(ApiError(anylive_common::AppError::validation(
            "receiver must be room owner",
        )));
    }

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

    // Fan-out only on first successful debit; idempotent replays must not re-broadcast.
    if !replayed {
        let channel = MessageEnvelope::room_channel(RoomId(room_id));
        let data = gift_envelope(
            order.id,
            order.room_id,
            order.sender_id,
            order.receiver_id,
            order.gift_id,
            order.count,
            order.total_coins,
        );
        if let Err(e) = state.centrifugo_publisher.publish(&channel, data).await {
            tracing::warn!(error = %e, %channel, "centrifugo gift publish failed");
        }
    }

    let status = if replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(GiftOrderDto::from_order(order, replayed))))
}
