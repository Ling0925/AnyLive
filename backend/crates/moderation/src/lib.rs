//! Admin moderation: ban users, force-close rooms, audit log.

use std::collections::HashSet;
use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{RoomId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub actor_id: UserId,
    pub action: String,
    pub target: String,
    pub detail: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Default)]
pub struct MemoryModeration {
    inner: Arc<Mutex<ModState>>,
}

#[derive(Default)]
struct ModState {
    banned_users: HashSet<Uuid>,
    /// user_id -> admin role
    admins: HashSet<Uuid>,
    audit: Vec<AuditEvent>,
}

impl MemoryModeration {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn grant_admin(&self, user_id: UserId) {
        self.inner.lock().await.admins.insert(user_id.0);
    }

    pub async fn is_admin(&self, user_id: UserId) -> bool {
        self.inner.lock().await.admins.contains(&user_id.0)
    }

    /// True when the admin set is empty (first-boot bootstrap window).
    pub async fn admin_count(&self) -> usize {
        self.inner.lock().await.admins.len()
    }

    pub async fn require_admin(&self, user_id: UserId) -> Result<(), AppError> {
        if self.is_admin(user_id).await {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::Forbidden, "admin only"))
        }
    }

    pub async fn ban_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let mut g = self.inner.lock().await;
        g.banned_users.insert(target.0);
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "ban_user".into(),
            target: target.0.to_string(),
            detail: reason.into(),
            created_at: Utc::now(),
        });
        Ok(())
    }

    pub async fn is_banned(&self, user_id: UserId) -> bool {
        self.inner.lock().await.banned_users.contains(&user_id.0)
    }

    pub async fn audit_force_close(
        &self,
        actor: UserId,
        room_id: RoomId,
        detail: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let mut g = self.inner.lock().await;
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "force_close_room".into(),
            target: room_id.0.to_string(),
            detail: detail.into(),
            created_at: Utc::now(),
        });
        Ok(())
    }

    pub async fn recent_audit(&self, limit: usize) -> Vec<AuditEvent> {
        let g = self.inner.lock().await;
        let limit = limit.clamp(1, 200);
        g.audit.iter().rev().take(limit).cloned().collect()
    }
}

/// Helper map for tests that need email->admin bootstrap is elsewhere.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ban_requires_admin() {
        let m = MemoryModeration::new();
        let admin = UserId::new();
        let user = UserId::new();
        assert_eq!(m.admin_count().await, 0);
        let err = m.ban_user(admin, user, "x").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
        m.grant_admin(admin).await;
        assert_eq!(m.admin_count().await, 1);
        m.ban_user(admin, user, "spam").await.unwrap();
        assert!(m.is_banned(user).await);
        let audit = m.recent_audit(10).await;
        assert_eq!(audit[0].action, "ban_user");
    }
}
