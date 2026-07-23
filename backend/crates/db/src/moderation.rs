//! [`PostgresModeration`] + dual [`AnyModeration`] matching [`MemoryModeration`] surface.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, UserId};
use anylive_moderation::{AdminRole, AuditEvent, MemoryModeration};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Postgres-backed moderation (admin_users, banned_users, muted_users, admin_audit).
#[derive(Clone)]
pub struct PostgresModeration {
    pool: PgPool,
}

impl PostgresModeration {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn grant_admin(&self, user_id: UserId) {
        self.grant_role(user_id, AdminRole::Admin).await;
    }

    pub async fn grant_role(&self, user_id: UserId, role: AdminRole) {
        let _ = sqlx::query(
            r#"
            INSERT INTO admin_users (user_id, role)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(user_id.0)
        .bind(role.as_str())
        .execute(&self.pool)
        .await;
    }

    /// Grant admin with audit event. Fails closed on DB errors.
    pub async fn grant_admin_audited(
        &self,
        actor: UserId,
        target: UserId,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        self.grant_role_audited(actor, target, AdminRole::Admin, detail)
            .await
    }

    pub async fn grant_role_audited(
        &self,
        actor: UserId,
        target: UserId,
        role: AdminRole,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        let detail = format!("{} role={}", detail.into(), role.as_str());
        let mut tx = self.pool.begin().await.map_err(map_db)?;
        sqlx::query(
            r#"
            INSERT INTO admin_users (user_id, role)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(target.0)
        .bind(role.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;
        push_audit(
            &mut tx,
            actor,
            "grant_admin",
            target.0.to_string(),
            detail,
        )
        .await?;
        tx.commit().await.map_err(map_db)?;
        Ok(())
    }

    /// Atomic bootstrap: insert only when no admins exist.
    /// Uses a single statement so concurrent bootstraps cannot both succeed.
    pub async fn try_bootstrap_admin(&self, user_id: UserId) -> Result<bool, AppError> {
        let res = sqlx::query(
            r#"
            INSERT INTO admin_users (user_id, role)
            SELECT $1, 'admin'
            WHERE (SELECT COUNT(*) FROM admin_users) = 0
            ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id.0)
        .execute(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn is_admin(&self, user_id: UserId) -> bool {
        match self.try_is_admin(user_id).await {
            Ok(v) => v,
            // Fail closed for privilege: treat DB errors as non-admin.
            Err(err) => {
                tracing::error!(error = %err, "postgres is_admin failed (fail-closed)");
                false
            }
        }
    }

    /// Fallible admin check — use when callers must distinguish DB failure.
    pub async fn try_is_admin(&self, user_id: UserId) -> Result<bool, AppError> {
        Ok(self.try_admin_role(user_id).await?.is_some())
    }

    pub async fn admin_role(&self, user_id: UserId) -> Option<AdminRole> {
        self.try_admin_role(user_id).await.ok().flatten()
    }

    pub async fn try_admin_role(&self, user_id: UserId) -> Result<Option<AdminRole>, AppError> {
        let row: Option<String> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(role, 'admin') FROM admin_users WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.and_then(|s| AdminRole::parse(&s)))
    }

    pub async fn admin_count(&self) -> usize {
        match self.try_admin_count().await {
            Ok(n) => n,
            // Fail closed for bootstrap: never report 0 on DB error (would reopen self-grant).
            Err(err) => {
                tracing::error!(error = %err, "postgres admin_count failed (fail-closed as max)");
                usize::MAX
            }
        }
    }

    /// Fallible admin count — use when callers must distinguish DB failure.
    pub async fn try_admin_count(&self) -> Result<usize, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_users")
            .fetch_one(&self.pool)
            .await
            .map_err(map_db)?;
        Ok(count.max(0) as usize)
    }

    pub async fn require_admin(&self, user_id: UserId) -> Result<(), AppError> {
        self.require_role(user_id, AdminRole::Admin).await.map(|_| ())
    }

    pub async fn require_role(
        &self,
        user_id: UserId,
        min: AdminRole,
    ) -> Result<AdminRole, AppError> {
        match self.try_admin_role(user_id).await? {
            Some(role) if role.meets(min) => Ok(role),
            Some(_) => Err(AppError::new(
                ErrorCode::Forbidden,
                format!("requires role {} or higher", min.as_str()),
            )),
            None => Err(AppError::new(ErrorCode::Forbidden, "admin only")),
        }
    }

    pub async fn ban_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let reason = reason.into();
        let mut tx = self.pool.begin().await.map_err(map_db)?;

        sqlx::query(
            r#"
            INSERT INTO banned_users (user_id, reason)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET reason = EXCLUDED.reason
            "#,
        )
        .bind(target.0)
        .bind(&reason)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        push_audit(&mut tx, actor, "ban_user", target.0.to_string(), reason).await?;
        tx.commit().await.map_err(map_db)?;
        Ok(())
    }

    pub async fn unban_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let reason = reason.into();
        let mut tx = self.pool.begin().await.map_err(map_db)?;

        sqlx::query("DELETE FROM banned_users WHERE user_id = $1")
            .bind(target.0)
            .execute(&mut *tx)
            .await
            .map_err(map_db)?;

        push_audit(&mut tx, actor, "unban_user", target.0.to_string(), reason).await?;
        tx.commit().await.map_err(map_db)?;
        Ok(())
    }

    pub async fn is_banned(&self, user_id: UserId) -> bool {
        match self.try_is_banned(user_id).await {
            Ok(v) => v,
            // Fail closed for policy: treat DB errors as banned so enforcement does not open.
            Err(err) => {
                tracing::error!(error = %err, "postgres is_banned failed (fail-closed as banned)");
                true
            }
        }
    }

    /// Fallible ban check — returns `AppError` on DB failure.
    pub async fn try_is_banned(&self, user_id: UserId) -> Result<bool, AppError> {
        let row: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT true FROM banned_users WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.unwrap_or(false))
    }

    pub async fn mute_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let reason = reason.into();
        let mut tx = self.pool.begin().await.map_err(map_db)?;

        sqlx::query(
            r#"
            INSERT INTO muted_users (user_id, reason)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET reason = EXCLUDED.reason
            "#,
        )
        .bind(target.0)
        .bind(&reason)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        push_audit(&mut tx, actor, "mute_user", target.0.to_string(), reason).await?;
        tx.commit().await.map_err(map_db)?;
        Ok(())
    }

    pub async fn unmute_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let reason = reason.into();
        let mut tx = self.pool.begin().await.map_err(map_db)?;

        sqlx::query(
            r#"
            DELETE FROM muted_users WHERE user_id = $1
            "#,
        )
        .bind(target.0)
        .execute(&mut *tx)
        .await
        .map_err(map_db)?;

        push_audit(
            &mut tx,
            actor,
            "unmute_user",
            target.0.to_string(),
            reason,
        )
        .await?;
        tx.commit().await.map_err(map_db)?;
        Ok(())
    }

    pub async fn is_muted(&self, user_id: UserId) -> bool {
        match self.try_is_muted(user_id).await {
            Ok(v) => v,
            // Fail closed for policy: treat DB errors as muted.
            Err(err) => {
                tracing::error!(error = %err, "postgres is_muted failed (fail-closed as muted)");
                true
            }
        }
    }

    /// Fallible mute check — returns `AppError` on DB failure.
    pub async fn try_is_muted(&self, user_id: UserId) -> Result<bool, AppError> {
        let row: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT true FROM muted_users WHERE user_id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db)?;
        Ok(row.unwrap_or(false))
    }

    pub async fn audit_force_close(
        &self,
        actor: UserId,
        room_id: RoomId,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let mut tx = self.pool.begin().await.map_err(map_db)?;
        push_audit(
            &mut tx,
            actor,
            "force_close_room",
            room_id.0.to_string(),
            detail.into(),
        )
        .await?;
        tx.commit().await.map_err(map_db)?;
        Ok(())
    }

    pub async fn recent_audit(&self, limit: usize) -> Vec<AuditEvent> {
        let limit = limit.clamp(1, 200) as i64;
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT id, actor_id, action, target, detail, created_at
            FROM admin_audit
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "postgres recent_audit failed");
            Vec::new()
        });
        rows.into_iter().map(AuditRow::into_event).collect()
    }
}

async fn push_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: UserId,
    action: &str,
    target: String,
    detail: String,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO admin_audit (id, actor_id, action, target, detail, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.0)
    .bind(action)
    .bind(target)
    .bind(detail)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(map_db)?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct AuditRow {
    id: Uuid,
    actor_id: Uuid,
    action: String,
    target: String,
    detail: String,
    created_at: DateTime<Utc>,
}

impl AuditRow {
    fn into_event(self) -> AuditEvent {
        AuditEvent {
            id: self.id,
            actor_id: UserId(self.actor_id),
            action: self.action,
            target: self.target,
            detail: self.detail,
            created_at: self.created_at,
        }
    }
}

/// Dual backend so the API can switch memory ↔ Postgres without generics on `AppState`.
#[derive(Clone)]
pub enum AnyModeration {
    Memory(MemoryModeration),
    Postgres(PostgresModeration),
}

impl AnyModeration {
    pub fn memory() -> Self {
        Self::Memory(MemoryModeration::new())
    }

    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(PostgresModeration::new(pool))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn grant_admin(&self, user_id: UserId) {
        match self {
            Self::Memory(m) => m.grant_admin(user_id).await,
            Self::Postgres(m) => m.grant_admin(user_id).await,
        }
    }

    pub async fn grant_role(&self, user_id: UserId, role: AdminRole) {
        match self {
            Self::Memory(m) => m.grant_role(user_id, role).await,
            Self::Postgres(m) => m.grant_role(user_id, role).await,
        }
    }

    pub async fn grant_admin_audited(
        &self,
        actor: UserId,
        target: UserId,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        let detail = detail.into();
        match self {
            Self::Memory(m) => {
                m.grant_admin_audited(actor, target, detail).await;
                Ok(())
            }
            Self::Postgres(m) => m.grant_admin_audited(actor, target, detail).await,
        }
    }

    pub async fn grant_role_audited(
        &self,
        actor: UserId,
        target: UserId,
        role: AdminRole,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        let detail = detail.into();
        match self {
            Self::Memory(m) => {
                m.grant_role_audited(actor, target, role, detail).await;
                Ok(())
            }
            Self::Postgres(m) => m.grant_role_audited(actor, target, role, detail).await,
        }
    }

    /// Atomic bootstrap grant. Returns true if this call created the first admin.
    pub async fn try_bootstrap_admin(&self, user_id: UserId) -> Result<bool, AppError> {
        match self {
            Self::Memory(m) => Ok(m.try_bootstrap_admin(user_id).await),
            Self::Postgres(m) => m.try_bootstrap_admin(user_id).await,
        }
    }

    pub async fn is_admin(&self, user_id: UserId) -> bool {
        match self {
            Self::Memory(m) => m.is_admin(user_id).await,
            Self::Postgres(m) => m.is_admin(user_id).await,
        }
    }

    pub async fn admin_role(&self, user_id: UserId) -> Option<AdminRole> {
        match self {
            Self::Memory(m) => m.admin_role(user_id).await,
            Self::Postgres(m) => m.admin_role(user_id).await,
        }
    }

    /// Fallible admin check (Postgres maps DB errors; memory always Ok).
    pub async fn try_is_admin(&self, user_id: UserId) -> Result<bool, AppError> {
        match self {
            Self::Memory(m) => Ok(m.is_admin(user_id).await),
            Self::Postgres(m) => m.try_is_admin(user_id).await,
        }
    }

    pub async fn admin_count(&self) -> usize {
        match self {
            Self::Memory(m) => m.admin_count().await,
            Self::Postgres(m) => m.admin_count().await,
        }
    }

    /// Fallible admin count (Postgres maps DB errors; memory always Ok).
    pub async fn try_admin_count(&self) -> Result<usize, AppError> {
        match self {
            Self::Memory(m) => Ok(m.admin_count().await),
            Self::Postgres(m) => m.try_admin_count().await,
        }
    }

    pub async fn require_admin(&self, user_id: UserId) -> Result<(), AppError> {
        match self {
            Self::Memory(m) => m.require_admin(user_id).await,
            Self::Postgres(m) => m.require_admin(user_id).await,
        }
    }

    pub async fn require_role(
        &self,
        user_id: UserId,
        min: AdminRole,
    ) -> Result<AdminRole, AppError> {
        match self {
            Self::Memory(m) => m.require_role(user_id, min).await,
            Self::Postgres(m) => m.require_role(user_id, min).await,
        }
    }

    pub async fn ban_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        match self {
            Self::Memory(m) => m.ban_user(actor, target, reason).await,
            Self::Postgres(m) => m.ban_user(actor, target, reason).await,
        }
    }

    pub async fn unban_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        match self {
            Self::Memory(m) => m.unban_user(actor, target, reason).await,
            Self::Postgres(m) => m.unban_user(actor, target, reason).await,
        }
    }

    pub async fn is_banned(&self, user_id: UserId) -> bool {
        match self {
            Self::Memory(m) => m.is_banned(user_id).await,
            Self::Postgres(m) => m.is_banned(user_id).await,
        }
    }

    /// Fallible ban check (Postgres maps DB errors; memory always Ok).
    pub async fn try_is_banned(&self, user_id: UserId) -> Result<bool, AppError> {
        match self {
            Self::Memory(m) => Ok(m.is_banned(user_id).await),
            Self::Postgres(m) => m.try_is_banned(user_id).await,
        }
    }

    pub async fn mute_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        match self {
            Self::Memory(m) => m.mute_user(actor, target, reason).await,
            Self::Postgres(m) => m.mute_user(actor, target, reason).await,
        }
    }

    pub async fn unmute_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        match self {
            Self::Memory(m) => m.unmute_user(actor, target, reason).await,
            Self::Postgres(m) => m.unmute_user(actor, target, reason).await,
        }
    }

    pub async fn is_muted(&self, user_id: UserId) -> bool {
        match self {
            Self::Memory(m) => m.is_muted(user_id).await,
            Self::Postgres(m) => m.is_muted(user_id).await,
        }
    }

    /// Fallible mute check (Postgres maps DB errors; memory always Ok).
    pub async fn try_is_muted(&self, user_id: UserId) -> Result<bool, AppError> {
        match self {
            Self::Memory(m) => Ok(m.is_muted(user_id).await),
            Self::Postgres(m) => m.try_is_muted(user_id).await,
        }
    }

    pub async fn audit_force_close(
        &self,
        actor: UserId,
        room_id: RoomId,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        match self {
            Self::Memory(m) => m.audit_force_close(actor, room_id, detail).await,
            Self::Postgres(m) => m.audit_force_close(actor, room_id, detail).await,
        }
    }

    pub async fn recent_audit(&self, limit: usize) -> Vec<AuditEvent> {
        match self {
            Self::Memory(m) => m.recent_audit(limit).await,
            Self::Postgres(m) => m.recent_audit(limit).await,
        }
    }
}

fn map_db(err: sqlx::Error) -> AppError {
    tracing::error!(error = %err, "postgres moderation store error");
    if let sqlx::Error::Database(db) = &err {
        if db.constraint().is_some_and(|c| c.ends_with("_fkey")) {
            return AppError::validation("referenced user does not exist");
        }
    }
    AppError::new(ErrorCode::Internal, "database error")
}

/// Pure SQL fragments (offline-testable, no live DB).
#[allow(dead_code)]
pub mod sql {
    pub const INSERT_ADMIN: &str = r#"
            INSERT INTO admin_users (user_id, role)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET role = EXCLUDED.role
            "#;

    pub const SELECT_IS_ADMIN: &str = r#"
            SELECT COALESCE(role, 'admin') FROM admin_users WHERE user_id = $1
            "#;

    pub const UPSERT_BANNED: &str = r#"
            INSERT INTO banned_users (user_id, reason)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET reason = EXCLUDED.reason
            "#;

    pub const DELETE_BANNED: &str = r#"
            DELETE FROM banned_users WHERE user_id = $1
            "#;

    pub const UPSERT_MUTED: &str = r#"
            INSERT INTO muted_users (user_id, reason)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET reason = EXCLUDED.reason
            "#;

    pub const DELETE_MUTED: &str = r#"
            DELETE FROM muted_users WHERE user_id = $1
            "#;

    pub const INSERT_AUDIT: &str = r#"
        INSERT INTO admin_audit (id, actor_id, action, target, detail, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#;

    pub const SELECT_RECENT_AUDIT: &str = r#"
            SELECT id, actor_id, action, target, detail, created_at
            FROM admin_audit
            ORDER BY created_at DESC
            LIMIT $1
            "#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres_enabled;
    use anylive_auth::UserStore;

    #[test]
    fn sql_fragments_cover_moderation_tables() {
        assert!(sql::INSERT_ADMIN.contains("admin_users"));
        assert!(sql::UPSERT_BANNED.contains("banned_users"));
        assert!(sql::UPSERT_MUTED.contains("muted_users"));
        assert!(sql::DELETE_MUTED.contains("DELETE FROM muted_users"));
        assert!(sql::INSERT_AUDIT.contains("admin_audit"));
        assert!(sql::SELECT_RECENT_AUDIT.contains("ORDER BY created_at DESC"));
    }

    #[tokio::test]
    async fn memory_try_helpers_match_bool_paths() {
        let m = AnyModeration::memory();
        let admin = UserId::new();
        let user = UserId::new();
        assert_eq!(m.try_admin_count().await.unwrap(), 0);
        assert!(!m.try_is_admin(admin).await.unwrap());
        m.grant_admin(admin).await;
        assert_eq!(m.try_admin_count().await.unwrap(), 1);
        assert!(m.try_is_admin(admin).await.unwrap());
        m.ban_user(admin, user, "spam").await.unwrap();
        assert!(m.try_is_banned(user).await.unwrap());
        m.mute_user(admin, user, "noise").await.unwrap();
        assert!(m.try_is_muted(user).await.unwrap());
    }

    #[tokio::test]
    async fn memory_backend_ban_and_mute() {
        let m = AnyModeration::memory();
        let admin = UserId::new();
        let user = UserId::new();
        assert_eq!(m.admin_count().await, 0);
        assert!(m.ban_user(admin, user, "x").await.is_err());
        m.grant_admin(admin).await;
        assert!(m.is_admin(admin).await);
        m.ban_user(admin, user, "spam").await.unwrap();
        assert!(m.is_banned(user).await);
        m.mute_user(admin, user, "noise").await.unwrap();
        assert!(m.is_muted(user).await);
        m.unmute_user(admin, user, "ok").await.unwrap();
        assert!(!m.is_muted(user).await);
        let audit = m.recent_audit(10).await;
        assert!(!audit.is_empty());
        assert!(!m.is_postgres());
    }

    /// Optional integration — skipped unless `USE_POSTGRES=1` + `DATABASE_URL`.
    #[tokio::test]
    async fn postgres_moderation_roundtrip() {
        if !postgres_enabled() {
            return;
        }
        let url = std::env::var("DATABASE_URL").unwrap();
        let pool = crate::connect(&url).await.expect("connect");
        crate::migrate(&pool).await.expect("migrate");

        let users = crate::PostgresUserStore::new(pool.clone());
        let admin = users
            .upsert_by_email(&format!("mod-admin-{}@example.com", Uuid::new_v4()))
            .await
            .expect("admin");
        let target = users
            .upsert_by_email(&format!("mod-target-{}@example.com", Uuid::new_v4()))
            .await
            .expect("target");

        let m = PostgresModeration::new(pool);
        assert!(m.ban_user(admin.id, target.id, "x").await.is_err());
        m.grant_admin(admin.id).await;
        assert!(m.is_admin(admin.id).await);
        assert!(m.admin_count().await >= 1);

        m.ban_user(admin.id, target.id, "spam").await.unwrap();
        assert!(m.is_banned(target.id).await);

        m.mute_user(admin.id, target.id, "chat spam").await.unwrap();
        assert!(m.is_muted(target.id).await);
        m.unmute_user(admin.id, target.id, "appeal").await.unwrap();
        assert!(!m.is_muted(target.id).await);

        let room = RoomId(Uuid::new_v4());
        m.audit_force_close(admin.id, room, "policy").await.unwrap();
        let audit = m.recent_audit(20).await;
        assert!(audit.iter().any(|e| e.action == "ban_user"));
        assert!(audit.iter().any(|e| e.action == "mute_user"));
        assert!(audit.iter().any(|e| e.action == "force_close_room"));
    }
}
