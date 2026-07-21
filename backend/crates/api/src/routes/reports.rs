//! User reports (moderation intake).

use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::UserId;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

/// Report workflow status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Open,
    Resolved,
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
        }
    }
}

impl Default for ReportStatus {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: UserId,
    pub target_type: String,
    pub target_id: String,
    pub reason: String,
    pub status: ReportStatus,
    /// Optional moderator note set when resolving.
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Default)]
pub struct MemoryReports {
    inner: Arc<Mutex<Vec<Report>>>,
}

impl MemoryReports {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn submit(
        &self,
        reporter: UserId,
        target_type: String,
        target_id: String,
        reason: String,
    ) -> Result<Report, AppError> {
        if reason.trim().is_empty() || reason.len() > 500 {
            return Err(AppError::validation("invalid reason"));
        }
        if !matches!(target_type.as_str(), "user" | "room" | "message") {
            return Err(AppError::validation("invalid target_type"));
        }
        let report = Report {
            id: Uuid::new_v4(),
            reporter_id: reporter,
            target_type,
            target_id,
            reason: reason.trim().to_string(),
            status: ReportStatus::Open,
            note: None,
            created_at: Utc::now(),
            resolved_at: None,
        };
        self.inner.lock().await.push(report.clone());
        Ok(report)
    }

    pub async fn list(&self, limit: usize) -> Vec<Report> {
        let g = self.inner.lock().await;
        g.iter().rev().take(limit.clamp(1, 100)).cloned().collect()
    }

    /// Resolve (or re-resolve) a report by id. Optional note is stored on the report.
    pub async fn resolve(
        &self,
        id: Uuid,
        note: Option<String>,
    ) -> Result<Report, AppError> {
        let mut g = self.inner.lock().await;
        let report = g
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::not_found("report not found"))?;
        if let Some(n) = note {
            let trimmed = n.trim().to_string();
            if trimmed.len() > 1000 {
                return Err(AppError::validation("note too long"));
            }
            report.note = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
        }
        report.status = ReportStatus::Resolved;
        report.resolved_at = Some(Utc::now());
        Ok(report.clone())
    }
}

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

    #[tokio::test]
    async fn submit_report() {
        let m = MemoryReports::new();
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
        let m = MemoryReports::new();
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
        let m = MemoryReports::new();
        let err = m.resolve(Uuid::new_v4(), None).await.unwrap_err();
        assert_eq!(err.code, anylive_common::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn resolve_without_note() {
        let m = MemoryReports::new();
        let r = m
            .submit(UserId::new(), "message".into(), "mid".into(), "spam".into())
            .await
            .unwrap();
        let resolved = m.resolve(r.id, None).await.unwrap();
        assert_eq!(resolved.status, ReportStatus::Resolved);
        assert!(resolved.note.is_none());
    }
}
