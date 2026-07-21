//! Admin gift catalog + report queue.

use std::sync::Arc;

use anylive_wallet::GiftCatalogItem;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::routes::wallet::GiftDto;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertGiftBody {
    pub name: String,
    pub price: i64,
    #[serde(default = "default_active")]
    pub active: bool,
    /// Optional fixed id for updates.
    #[serde(default)]
    pub id: Option<String>,
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminGiftListResponse {
    pub items: Vec<GiftDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminReportDto {
    pub id: String,
    pub reporter_id: String,
    pub target_type: String,
    pub target_id: String,
    pub reason: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminReportListResponse {
    pub items: Vec<AdminReportDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveReportBody {
    /// Must be `"resolved"`.
    pub status: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn report_to_dto(r: crate::routes::reports::Report) -> AdminReportDto {
    AdminReportDto {
        id: r.id.to_string(),
        reporter_id: r.reporter_id.0.to_string(),
        target_type: r.target_type,
        target_id: r.target_id,
        reason: r.reason,
        status: r.status.as_str().to_string(),
        note: r.note,
        created_at: r.created_at.to_rfc3339(),
    }
}

/// GET /api/v1/admin/gifts
#[utoipa::path(
    get,
    path = "/api/v1/admin/gifts",
    tag = "admin",
    security(("bearerAuth" = [])),
    responses((status = 200, body = AdminGiftListResponse))
)]
pub async fn admin_list_gifts(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<AdminGiftListResponse>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let items = state.wallet.list_gifts().await;
    Ok(Json(AdminGiftListResponse {
        items: items.into_iter().map(GiftDto::from).collect(),
    }))
}

/// POST /api/v1/admin/gifts
#[utoipa::path(
    post,
    path = "/api/v1/admin/gifts",
    tag = "admin",
    security(("bearerAuth" = [])),
    request_body = UpsertGiftBody,
    responses((status = 201, body = GiftDto))
)]
pub async fn admin_upsert_gift(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<UpsertGiftBody>,
) -> Result<(StatusCode, Json<GiftDto>), ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    if body.name.trim().is_empty() || body.price <= 0 {
        return Err(ApiError(anylive_common::AppError::validation(
            "invalid gift",
        )));
    }
    let id = match body.id.as_deref() {
        Some(s) => Uuid::parse_str(s)
            .map_err(|_| ApiError(anylive_common::AppError::validation("invalid gift id")))?,
        None => Uuid::new_v4(),
    };
    let item = GiftCatalogItem {
        id,
        name: body.name.trim().to_string(),
        price: body.price,
        active: body.active,
    };
    state.wallet.upsert_gift(item.clone()).await;
    Ok((StatusCode::CREATED, Json(GiftDto::from(item))))
}

/// GET /api/v1/admin/reports
#[utoipa::path(
    get,
    path = "/api/v1/admin/reports",
    tag = "admin",
    security(("bearerAuth" = [])),
    responses((status = 200, body = AdminReportListResponse))
)]
pub async fn admin_list_reports(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<AdminReportListResponse>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    let items = state.reports.list(100).await;
    Ok(Json(AdminReportListResponse {
        items: items.into_iter().map(report_to_dto).collect(),
    }))
}

/// PATCH /api/v1/admin/reports/{id}
#[utoipa::path(
    patch,
    path = "/api/v1/admin/reports/{id}",
    tag = "admin",
    security(("bearerAuth" = [])),
    params(("id" = String, Path, description = "Report UUID")),
    request_body = ResolveReportBody,
    responses((status = 200, body = AdminReportDto))
)]
pub async fn admin_resolve_report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ResolveReportBody>,
) -> Result<Json<AdminReportDto>, ApiError> {
    state
        .moderation
        .require_admin(user.user_id)
        .await
        .map_err(ApiError::from)?;
    if body.status != "resolved" {
        return Err(ApiError(anylive_common::AppError::validation(
            "status must be resolved",
        )));
    }
    let report_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError(anylive_common::AppError::validation("invalid report id")))?;
    let report = state
        .reports
        .resolve(report_id, body.note)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(report_to_dto(report)))
}
