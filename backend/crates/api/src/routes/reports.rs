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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: UserId,
    pub target_type: String,
    pub target_id: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
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
            created_at: Utc::now(),
        };
        self.inner.lock().await.push(report.clone());
        Ok(report)
    }

    pub async fn list(&self, limit: usize) -> Vec<Report> {
        let g = self.inner.lock().await;
        g.iter().rev().take(limit.clamp(1, 100)).cloned().collect()
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
        assert_eq!(m.list(10).await.len(), 1);
    }
}
