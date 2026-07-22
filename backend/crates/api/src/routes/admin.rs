//! Admin moderation HTTP handlers.

use std::sync::Arc;

use anylive_domain::{RoomId, RoomStatus, UserId};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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

    state
        .moderation
        .grant_admin_audited(user.user_id, UserId(target), "admin_grant")
        .await
        .map_err(ApiError)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Ban a user (admin).
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
