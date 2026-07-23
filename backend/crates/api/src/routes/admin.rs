//! Admin moderation HTTP handlers.

use std::sync::Arc;

use anylive_domain::{RoomId, RoomStatus, UserId};
use anylive_pay::PayStore;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use anylive_auth::RefreshStore;
use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::routes::rooms::RoomDto;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BanUserBody {
    pub user_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MuteUserBody {
    pub user_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UnmuteUserBody {
    pub user_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForceCloseBody {
    pub room_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GrantAdminBody {
    /// Bootstrap: only works when no admins exist, or caller is already admin.
    pub user_id: String,
    /// Optional role: admin | moderator | ops (default admin). Bootstrap always admin.
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventDto {
    pub id: String,
    pub actor_id: String,
    pub action: String,
    pub target: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditListResponse {
    pub items: Vec<AuditEventDto>,
}

/// Bootstrap/grant admin role.
///
/// First-boot only: any authenticated user may grant **themselves** while the
/// admin set is empty (atomic insert). After that, only existing admins may grant
/// others. Every grant is audited.
#[utoipa::path(post, path = "/api/v1/admin/grant", tag = "admin", security(("bearerAuth" = [])), request_body = GrantAdminBody, responses((status = 204)))]
pub async fn grant_admin(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<GrantAdminBody>,
) -> Result<StatusCode, ApiError> {
    let target = Uuid::parse_str(&body.user_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid user_id")))?;
    let is_self = target == user.user_id.0;
    // Fallible counts/checks: DB errors must not reopen the bootstrap window.
    let admin_count = state
        .moderation
        .try_admin_count()
        .await
        .map_err(ApiError)?;
    let caller_is_admin = state
        .moderation
        .try_is_admin(user.user_id)
        .await
        .map_err(ApiError)?;

    if admin_count == 0 {
        // Bootstrap window: only self-grant is allowed (prevents granting a peer
        // before any admin exists via a stolen session of a non-admin).
        if !is_self {
            return Err(ApiError(anylive_common::AppError::new(
                anylive_common::ErrorCode::Forbidden,
                "bootstrap may only grant the caller",
            )));
        }
        // Atomic check-and-insert so concurrent true-zero bootstraps cannot both win.
        let granted = state
            .moderation
            .try_bootstrap_admin(UserId(target))
            .await
            .map_err(ApiError)?;
        if !granted {
            return Err(ApiError(anylive_common::AppError::new(
                anylive_common::ErrorCode::Conflict,
                "admin bootstrap already claimed",
            )));
        }
        state
            .moderation
            .grant_admin_audited(user.user_id, UserId(target), "bootstrap")
            .await
            .map_err(ApiError)?;
        // grant_admin_audited re-inserts (idempotent) + audits.
        return Ok(StatusCode::NO_CONTENT);
    } else if !caller_is_admin {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::Forbidden,
            "admin only",
        )));
    }

    // Non-bootstrap grants require full admin; optional role (default admin).
    let role = match body.role.as_deref() {
        None | Some("") => anylive_moderation::AdminRole::Admin,
        Some(raw) => anylive_moderation::AdminRole::parse(raw).ok_or_else(|| {
            ApiError(anylive_common::AppError::validation(
                "role must be admin, moderator, or ops",
            ))
        })?,
    };
    // Only full admins may grant (caller already checked is_admin; re-check rank).
    state
        .moderation
        .require_role(user.user_id, anylive_moderation::AdminRole::Admin)
        .await
        .map_err(ApiError)?;
    state
        .moderation
        .grant_role_audited(user.user_id, UserId(target), role, "admin_grant")
        .await
        .map_err(ApiError)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Ban a user (admin). Also revokes all refresh sessions.
#[utoipa::path(post, path = "/api/v1/admin/ban", tag = "admin", security(("bearerAuth" = [])), request_body = BanUserBody, responses((status = 204)))]
pub async fn ban_user(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<BanUserBody>,
) -> Result<StatusCode, ApiError> {
    let target = Uuid::parse_str(&body.user_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid user_id")))?;
    state
        .moderation
        .ban_user(
            user.user_id,
            UserId(target),
            body.reason.unwrap_or_else(|| "policy".into()),
        )
        .await
        .map_err(ApiError::from)?;
    // Kick all sessions so banned users cannot keep using access tokens via refresh.
    let _ = state
        .auth
        .refresh_store()
        .revoke_all_for_user(UserId(target))
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Mute a user (admin). Blocks chat + gifts while leaving account active.
#[utoipa::path(post, path = "/api/v1/admin/mute", tag = "admin", security(("bearerAuth" = [])), request_body = MuteUserBody, responses((status = 204)))]
pub async fn mute_user(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<MuteUserBody>,
) -> Result<StatusCode, ApiError> {
    let target = Uuid::parse_str(&body.user_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid user_id")))?;
    state
        .moderation
        .mute_user(
            user.user_id,
            UserId(target),
            body.reason.unwrap_or_else(|| "policy".into()),
        )
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Unmute a user (admin).
#[utoipa::path(post, path = "/api/v1/admin/unmute", tag = "admin", security(("bearerAuth" = [])), request_body = UnmuteUserBody, responses((status = 204)))]
pub async fn unmute_user(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<UnmuteUserBody>,
) -> Result<StatusCode, ApiError> {
    let target = Uuid::parse_str(&body.user_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid user_id")))?;
    state
        .moderation
        .unmute_user(
            user.user_id,
            UserId(target),
            body.reason.unwrap_or_else(|| "policy".into()),
        )
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Force-close a room (admin).
#[utoipa::path(post, path = "/api/v1/admin/rooms/force-close", tag = "admin", security(("bearerAuth" = [])), request_body = ForceCloseBody, responses((status = 200, body = RoomDto)))]
pub async fn force_close_room(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<ForceCloseBody>,
) -> Result<Json<RoomDto>, ApiError> {
    let room_uuid = Uuid::parse_str(&body.room_id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid room_id")))?;
    let room_id = RoomId(room_uuid);
    state
        .moderation
        .audit_force_close(
            user.user_id,
            room_id,
            body.reason.unwrap_or_else(|| "admin force close".into()),
        )
        .await
        .map_err(ApiError::from)?;
    let room = state
        .rooms
        .force_close(room_id, None)
        .await
        .map_err(ApiError::from)?;
    state.media.clear_active_stream(room_id).await;
    // If already closed, force_close may error on transition — domain allows Idle|Live -> Closed.
    let _ = RoomStatus::Closed;
    Ok(Json(room.into()))
}

/// Recent audit events (admin).
#[utoipa::path(get, path = "/api/v1/admin/audit", tag = "admin", security(("bearerAuth" = [])), responses((status = 200, body = AuditListResponse)))]
pub async fn list_audit(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<AuditListResponse>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let items = state.moderation.recent_audit(50).await;
    Ok(Json(AuditListResponse {
        items: items
            .into_iter()
            .map(|e| AuditEventDto {
                id: e.id.to_string(),
                actor_id: e.actor_id.0.to_string(),
                action: e.action,
                target: e.target,
                detail: e.detail,
                created_at: e.created_at.to_rfc3339(),
            })
            .collect(),
    }))
}

/// Cap mismatch rows returned in a single reconcile response.
const RECONCILE_MISMATCH_LIMIT: usize = 50;

#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceMismatchDto {
    pub user_id: String,
    pub stored_balance: i64,
    pub ledger_sum: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletReconcileResponse {
    pub checked_users: u64,
    pub imbalance_count: u64,
    pub balanced: bool,
    pub mismatches: Vec<BalanceMismatchDto>,
    pub mismatches_truncated: bool,
}

/// Run wallet ledger/balance consistency scan (admin).
///
/// Compares each user's stored balance to Σ ledger amounts. P1 dogfood gate:
/// `imbalance_count == 0` after gift/topup flows.
#[utoipa::path(
    get,
    path = "/api/v1/admin/wallet/reconcile",
    tag = "admin",
    security(("bearerAuth" = [])),
    responses((status = 200, body = WalletReconcileResponse))
)]
pub async fn wallet_reconcile(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<WalletReconcileResponse>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let report = state.wallet.reconcile().await;
    let mut mismatches: Vec<BalanceMismatchDto> = report
        .mismatches
        .into_iter()
        .map(|m| BalanceMismatchDto {
            user_id: m.user_id.0.to_string(),
            stored_balance: m.stored_balance,
            ledger_sum: m.ledger_sum,
        })
        .collect();
    let mismatches_truncated = mismatches.len() > RECONCILE_MISMATCH_LIMIT;
    mismatches.truncate(RECONCILE_MISMATCH_LIMIT);
    Ok(Json(WalletReconcileResponse {
        checked_users: report.checked_users,
        imbalance_count: report.imbalance_count,
        balanced: report.imbalance_count == 0,
        mismatches,
        mismatches_truncated,
    }))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExpirePayOrdersResponse {
    pub expired_count: u64,
}

/// Expire unpaid pay orders past their `expires_at` (admin / cron hook).
#[utoipa::path(
    post,
    path = "/api/v1/admin/pay/expire-orders",
    tag = "admin",
    security(("bearerAuth" = [])),
    responses((status = 200, body = ExpirePayOrdersResponse))
)]
pub async fn expire_pay_orders(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<ExpirePayOrdersResponse>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let expired_count = state.pay.expire_stale_orders(chrono::Utc::now()).await;
    Ok(Json(ExpirePayOrdersResponse { expired_count }))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsNameCountDto {
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsSummaryResponse {
    /// Events currently retained in the process ring buffer.
    pub retained_events: u64,
    /// Distinct users among retained events (not true DAU).
    pub distinct_users: u64,
    pub by_name: Vec<AnalyticsNameCountDto>,
    pub recent: Vec<AnalyticsRecentEventDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsRecentEventDto {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub occurred_at: String,
    pub received_at: String,
}

/// Dogfood analytics summary from the in-memory client event buffer (P4).
///
/// Not a production warehouse / DAU dashboard — use for verifying ingest during
/// soft launch. Full DAU/paid boards remain external.
#[utoipa::path(
    get,
    path = "/api/v1/admin/analytics/summary",
    tag = "admin",
    security(("bearerAuth" = [])),
    responses((status = 200, body = AnalyticsSummaryResponse))
)]
pub async fn analytics_summary(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<AnalyticsSummaryResponse>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let retained = state.analytics.count().await as u64;
    let distinct_users = state.analytics.distinct_users().await;
    let by_name = state
        .analytics
        .counts_by_name()
        .await
        .into_iter()
        .map(|(name, count)| AnalyticsNameCountDto { name, count })
        .collect();
    let recent = state
        .analytics
        .recent(20)
        .await
        .into_iter()
        .map(|e| AnalyticsRecentEventDto {
            id: e.id.to_string(),
            user_id: e.user_id.0.to_string(),
            name: e.name,
            occurred_at: e.occurred_at.to_rfc3339(),
            received_at: e.received_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(AnalyticsSummaryResponse {
        retained_events: retained,
        distinct_users,
        by_name,
        recent,
    }))
}
