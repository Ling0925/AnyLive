//! User reports (moderation intake).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

// Re-export dual-store types used by AppState / admin routes / tests.
pub use anylive_db::{MemoryReports, Report, ReportStatus};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReportBody {
    pub target_type: String,
    pub target_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReportDto {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub reason: String,
    pub status: String,
    pub created_at: String,
}

/// POST /api/v1/reports
#[utoipa::path(
    post,
    path = "/api/v1/reports",
    tag = "moderation",
    security(("bearerAuth" = [])),
    request_body = CreateReportBody,
    responses((status = 201, body = ReportDto))
)]
pub async fn create_report(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<CreateReportBody>,
) -> Result<(StatusCode, Json<ReportDto>), ApiError> {
    let r = state
        .reports
        .submit(user.user_id, body.target_type, body.target_id, body.reason)
        .await
        .map_err(ApiError::from)?;
    Ok((
        StatusCode::CREATED,
        Json(ReportDto {
            id: r.id.to_string(),
            target_type: r.target_type,
            target_id: r.target_id,
            reason: r.reason,
            status: r.status.as_str().to_string(),
            created_at: r.created_at.to_rfc3339(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anylive_db::AnyReports;
    use anylive_domain::UserId;
    use uuid::Uuid;

    #[tokio::test]
    async fn submit_report() {
        let m = AnyReports::memory();
        let r = m
            .submit(UserId::new(), "room".into(), "rid".into(), "spam".into())
            .await
            .unwrap();
        assert_eq!(r.reason, "spam");
        assert_eq!(r.status, ReportStatus::Open);
        assert_eq!(m.list(10).await.len(), 1);
    }

    #[tokio::test]
    async fn resolve_report() {
        let m = AnyReports::memory();
        let r = m
            .submit(UserId::new(), "user".into(), "uid".into(), "abuse".into())
            .await
            .unwrap();
        assert_eq!(r.status, ReportStatus::Open);

        let resolved = m
            .resolve(r.id, Some("action taken".into()))
            .await
            .unwrap();
        assert_eq!(resolved.status, ReportStatus::Resolved);
        assert_eq!(resolved.note.as_deref(), Some("action taken"));
        assert!(resolved.resolved_at.is_some());

        let listed = m.list(10).await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, ReportStatus::Resolved);
        assert_eq!(listed[0].note.as_deref(), Some("action taken"));
    }

    #[tokio::test]
    async fn resolve_missing_report_not_found() {
        let m = AnyReports::memory();
        let err = m.resolve(Uuid::new_v4(), None).await.unwrap_err();
        assert_eq!(err.code, anylive_common::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn resolve_without_note() {
        let m = AnyReports::memory();
        let r = m
            .submit(UserId::new(), "message".into(), "mid".into(), "spam".into())
            .await
            .unwrap();
        let resolved = m.resolve(r.id, None).await.unwrap();
        assert_eq!(resolved.status, ReportStatus::Resolved);
        assert!(resolved.note.is_none());
    }
}
