//! Reports dual store: [`MemoryReports`], [`PostgresReports`], [`AnyReports`].

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

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

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "resolved" => Some(Self::Resolved),
            _ => None,
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
        validate_submit(&target_type, &reason)?;
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
    pub async fn resolve(&self, id: Uuid, note: Option<String>) -> Result<Report, AppError> {
        let mut g = self.inner.lock().await;
        let report = g
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::not_found("report not found"))?;
        apply_resolve(report, note)?;
        Ok(report.clone())
    }
}

/// Postgres-backed reports (`reports` table from `002_reports_mute.sql`).
#[derive(Clone)]
pub struct PostgresReports {
    pool: PgPool,
}

impl PostgresReports {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn submit(
        &self,
        reporter: UserId,
        target_type: String,
        target_id: String,
        reason: String,
    ) -> Result<Report, AppError> {
        validate_submit(&target_type, &reason)?;
        let reason = reason.trim().to_string();
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let row = sqlx::query_as::<_, ReportRow>(
            r#"
            INSERT INTO reports (
                id, reporter_id, target_type, target_id, reason,
                status, note, created_at, resolved_at
            )
            VALUES ($1, $2, $3, $4, $5, 'open', NULL, $6, NULL)
            RETURNING id, reporter_id, target_type, target_id, reason,
                      status, note, created_at, resolved_at
            "#,
        )
        .bind(id)
        .bind(reporter.0)
        .bind(&target_type)
        .bind(&target_id)
        .bind(&reason)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        row.into_report()
    }

    pub async fn list(&self, limit: usize) -> Vec<Report> {
        let limit = limit.clamp(1, 100) as i64;
        let rows = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT id, reporter_id, target_type, target_id, reason,
                   status, note, created_at, resolved_at
            FROM reports
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "postgres reports list failed");
            Vec::new()
        });
        rows.into_iter()
            .filter_map(|r| r.into_report().ok())
            .collect()
    }

    pub async fn resolve(&self, id: Uuid, note: Option<String>) -> Result<Report, AppError> {
        // Load first so we can apply the same note rules as memory.
        let mut report = sqlx::query_as::<_, ReportRow>(
            r#"
            SELECT id, reporter_id, target_type, target_id, reason,
                   status, note, created_at, resolved_at
            FROM reports
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?
        .ok_or_else(|| AppError::not_found("report not found"))?
        .into_report()?;

        apply_resolve(&mut report, note)?;

        let row = sqlx::query_as::<_, ReportRow>(
            r#"
            UPDATE reports
            SET status = $2, note = $3, resolved_at = $4
            WHERE id = $1
            RETURNING id, reporter_id, target_type, target_id, reason,
                      status, note, created_at, resolved_at
            "#,
        )
        .bind(report.id)
        .bind(report.status.as_str())
        .bind(&report.note)
        .bind(report.resolved_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db)?;
        row.into_report()
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyReports {
    Memory(MemoryReports),
    Postgres(PostgresReports),
}

impl AnyReports {
    pub fn memory() -> Self {
        Self::Memory(MemoryReports::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresReports::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn submit(
        &self,
        reporter: UserId,
        target_type: String,
        target_id: String,
        reason: String,
    ) -> Result<Report, AppError> {
        match self {
            Self::Memory(r) => r.submit(reporter, target_type, target_id, reason).await,
            Self::Postgres(r) => r.submit(reporter, target_type, target_id, reason).await,
        }
    }

    pub async fn list(&self, limit: usize) -> Vec<Report> {
        match self {
            Self::Memory(r) => r.list(limit).await,
            Self::Postgres(r) => r.list(limit).await,
        }
    }

    pub async fn resolve(&self, id: Uuid, note: Option<String>) -> Result<Report, AppError> {
        match self {
            Self::Memory(r) => r.resolve(id, note).await,
            Self::Postgres(r) => r.resolve(id, note).await,
        }
    }
}

fn validate_submit(target_type: &str, reason: &str) -> Result<(), AppError> {
    if reason.trim().is_empty() || reason.len() > 500 {
        return Err(AppError::validation("invalid reason"));
    }
    if !matches!(target_type, "user" | "room" | "message") {
        return Err(AppError::validation("invalid target_type"));
    }
    Ok(())
}

fn apply_resolve(report: &mut Report, note: Option<String>) -> Result<(), AppError> {
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
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct ReportRow {
    id: Uuid,
    reporter_id: Uuid,
    target_type: String,
    target_id: String,
    reason: String,
    status: String,
    note: Option<String>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl ReportRow {
    fn into_report(self) -> Result<Report, AppError> {
        let status = ReportStatus::parse(&self.status).ok_or_else(|| {
            AppError::new(
                ErrorCode::Internal,
                format!("invalid report status in db: {}", self.status),
            )
        })?;
        Ok(Report {
            id: self.id,
            reporter_id: UserId(self.reporter_id),
            target_type: self.target_type,
            target_id: self.target_id,
            reason: self.reason,
            status,
            note: self.note,
            created_at: self.created_at,
            resolved_at: self.resolved_at,
        })
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres reports store error");
    if let sqlx::Error::Database(db) = &err {
        if db.constraint().is_some_and(|c| c.ends_with("_fkey")) {
            return AppError::validation("reporter user does not exist");
        }
    }
    AppError::new(ErrorCode::Internal, "database error")
}

/// Pure SQL fragments (offline-testable, no live DB).
#[allow(dead_code)]
pub mod sql {
    pub const INSERT_REPORT: &str = r#"
            INSERT INTO reports (
                id, reporter_id, target_type, target_id, reason,
                status, note, created_at, resolved_at
            )
            VALUES ($1, $2, $3, $4, $5, 'open', NULL, $6, NULL)
            RETURNING id, reporter_id, target_type, target_id, reason,
                      status, note, created_at, resolved_at
            "#;

    pub const LIST_REPORTS: &str = r#"
            SELECT id, reporter_id, target_type, target_id, reason,
                   status, note, created_at, resolved_at
            FROM reports
            ORDER BY created_at DESC
            LIMIT $1
            "#;

    pub const UPDATE_RESOLVE: &str = r#"
            UPDATE reports
            SET status = $2, note = $3, resolved_at = $4
            WHERE id = $1
            RETURNING id, reporter_id, target_type, target_id, reason,
                      status, note, created_at, resolved_at
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[test]
    fn sql_fragments_mention_reports() {
        assert!(sql::INSERT_REPORT.contains("INSERT INTO reports"));
        assert!(sql::LIST_REPORTS.contains("ORDER BY created_at DESC"));
        assert!(sql::UPDATE_RESOLVE.contains("SET status = $2"));
    }

    #[tokio::test]
    async fn memory_backend_submit_and_resolve() {
        let m = AnyReports::memory();
        let r = m
            .submit(UserId::new(), "room".into(), "rid".into(), "spam".into())
            .await
            .unwrap();
        assert_eq!(r.reason, "spam");
        assert_eq!(r.status, ReportStatus::Open);
        assert_eq!(m.list(10).await.len(), 1);

        let resolved = m
            .resolve(r.id, Some("action taken".into()))
            .await
            .unwrap();
        assert_eq!(resolved.status, ReportStatus::Resolved);
        assert_eq!(resolved.note.as_deref(), Some("action taken"));
        assert!(resolved.resolved_at.is_some());
        assert!(!m.is_postgres());
    }

    #[tokio::test]
    async fn memory_resolve_missing_not_found() {
        let m = AnyReports::memory();
        let err = m.resolve(Uuid::new_v4(), None).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn memory_reject_invalid_target() {
        let m = AnyReports::memory();
        let err = m
            .submit(UserId::new(), "widget".into(), "x".into(), "spam".into())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_reports_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let reporter = crate::PostgresUserStore::new(pool.clone())
            .upsert_by_email(&format!("report-{}@example.com", Uuid::new_v4()))
            .await
            .expect("reporter");

        let store = PostgresReports::new(pool);
        let r = store
            .submit(
                reporter.id,
                "user".into(),
                reporter.id.0.to_string(),
                "abuse".into(),
            )
            .await
            .unwrap();
        assert_eq!(r.status, ReportStatus::Open);

        let listed = store.list(10).await;
        assert!(listed.iter().any(|x| x.id == r.id));

        let resolved = store
            .resolve(r.id, Some("warned".into()))
            .await
            .unwrap();
        assert_eq!(resolved.status, ReportStatus::Resolved);
        assert_eq!(resolved.note.as_deref(), Some("warned"));
        assert!(resolved.resolved_at.is_some());
    }
}
