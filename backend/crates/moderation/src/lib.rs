//! Admin moderation: ban/mute users, force-close rooms, audit log.

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

/// One row from the ban/mute lists (ops console).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModerationEntry {
    pub user_id: UserId,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

/// Combined ban/mute flags for a single user lookup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UserModerationStatus {
    pub user_id: UserId,
    pub banned: bool,
    pub muted: bool,
    pub ban_reason: Option<String>,
    pub mute_reason: Option<String>,
    pub banned_at: Option<DateTime<Utc>>,
    pub muted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct Sanction {
    reason: String,
    created_at: DateTime<Utc>,
}

/// Admin RBAC roles (WBS E7.1).
///
/// - `admin` — full control plane (grant, ban, force-close, pay, wallet)
/// - `moderator` — content actions (mute, reports, gifts catalog)
/// - `ops` — read-mostly ops (audit, analytics)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdminRole {
    #[default]
    Admin,
    Moderator,
    Ops,
}

impl AdminRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Moderator => "moderator",
            Self::Ops => "ops",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Self::Admin),
            "moderator" | "mod" => Some(Self::Moderator),
            "ops" | "operator" => Some(Self::Ops),
            _ => None,
        }
    }

    /// Privilege rank: higher can do everything lower can.
    pub fn rank(self) -> u8 {
        match self {
            Self::Ops => 1,
            Self::Moderator => 2,
            Self::Admin => 3,
        }
    }

    pub fn meets(self, required: AdminRole) -> bool {
        self.rank() >= required.rank()
    }
}

#[derive(Clone, Default)]
pub struct MemoryModeration {
    inner: Arc<Mutex<ModState>>,
}

#[derive(Default)]
struct ModState {
    /// Global ban (P1: no expiry).
    banned_users: std::collections::HashMap<Uuid, Sanction>,
    /// Global mute (P1: no expiry). Muted users cannot chat or send gifts.
    muted_users: std::collections::HashMap<Uuid, Sanction>,
    /// user_id -> admin role (presence = is_admin for backward compat)
    admins: std::collections::HashMap<Uuid, AdminRole>,
    audit: Vec<AuditEvent>,
}

impl MemoryModeration {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn grant_admin(&self, user_id: UserId) {
        self.grant_role(user_id, AdminRole::Admin).await;
    }

    /// Grant a specific admin role (upsert).
    pub async fn grant_role(&self, user_id: UserId, role: AdminRole) {
        self.inner.lock().await.admins.insert(user_id.0, role);
    }

    /// Grant admin and write an audit event (actor, target, bootstrap vs admin path).
    pub async fn grant_admin_audited(
        &self,
        actor: UserId,
        target: UserId,
        detail: impl Into<String>,
    ) {
        self.grant_role_audited(actor, target, AdminRole::Admin, detail)
            .await;
    }

    /// Grant role with audit event.
    pub async fn grant_role_audited(
        &self,
        actor: UserId,
        target: UserId,
        role: AdminRole,
        detail: impl Into<String>,
    ) {
        let mut g = self.inner.lock().await;
        g.admins.insert(target.0, role);
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "grant_admin".into(),
            target: target.0.to_string(),
            detail: format!("{} role={}", detail.into(), role.as_str()),
            created_at: Utc::now(),
        });
    }

    /// Atomic bootstrap: insert only when admin set is empty. Returns true if granted.
    pub async fn try_bootstrap_admin(&self, user_id: UserId) -> bool {
        let mut g = self.inner.lock().await;
        if !g.admins.is_empty() {
            return false;
        }
        g.admins.insert(user_id.0, AdminRole::Admin);
        true
    }

    pub async fn is_admin(&self, user_id: UserId) -> bool {
        self.inner.lock().await.admins.contains_key(&user_id.0)
    }

    /// Role for a staff user, if any.
    pub async fn admin_role(&self, user_id: UserId) -> Option<AdminRole> {
        self.inner.lock().await.admins.get(&user_id.0).copied()
    }

    /// True when the admin set is empty (first-boot bootstrap window).
    pub async fn admin_count(&self) -> usize {
        self.inner.lock().await.admins.len()
    }

    pub async fn require_admin(&self, user_id: UserId) -> Result<(), AppError> {
        self.require_role(user_id, AdminRole::Admin)
            .await
            .map(|_| ())
    }

    /// Require at least `min` role rank (admin > moderator > ops).
    pub async fn require_role(
        &self,
        user_id: UserId,
        min: AdminRole,
    ) -> Result<AdminRole, AppError> {
        match self.admin_role(user_id).await {
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
        let mut g = self.inner.lock().await;
        g.banned_users.insert(
            target.0,
            Sanction {
                reason: reason.clone(),
                created_at: Utc::now(),
            },
        );
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "ban_user".into(),
            target: target.0.to_string(),
            detail: reason,
            created_at: Utc::now(),
        });
        Ok(())
    }

    /// Remove a ban (admin). Idempotent when the user is not banned.
    pub async fn unban_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let mut g = self.inner.lock().await;
        g.banned_users.remove(&target.0);
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "unban_user".into(),
            target: target.0.to_string(),
            detail: reason.into(),
            created_at: Utc::now(),
        });
        Ok(())
    }

    pub async fn is_banned(&self, user_id: UserId) -> bool {
        self.inner.lock().await.banned_users.contains_key(&user_id.0)
    }

    /// Mute a user (admin). Global mute for P1 — blocks chat + gifts.
    pub async fn mute_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let reason = reason.into();
        let mut g = self.inner.lock().await;
        g.muted_users.insert(
            target.0,
            Sanction {
                reason: reason.clone(),
                created_at: Utc::now(),
            },
        );
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "mute_user".into(),
            target: target.0.to_string(),
            detail: reason,
            created_at: Utc::now(),
        });
        Ok(())
    }

    /// Unmute a user (admin).
    pub async fn unmute_user(
        &self,
        actor: UserId,
        target: UserId,
        reason: impl Into<String>,
    ) -> Result<(), AppError> {
        self.require_admin(actor).await?;
        let mut g = self.inner.lock().await;
        g.muted_users.remove(&target.0);
        g.audit.push(AuditEvent {
            id: Uuid::new_v4(),
            actor_id: actor,
            action: "unmute_user".into(),
            target: target.0.to_string(),
            detail: reason.into(),
            created_at: Utc::now(),
        });
        Ok(())
    }

    pub async fn is_muted(&self, user_id: UserId) -> bool {
        self.inner.lock().await.muted_users.contains_key(&user_id.0)
    }

    /// Banned users newest-first (limit clamped 1..=200).
    pub async fn list_banned(&self, limit: usize) -> Vec<ModerationEntry> {
        let g = self.inner.lock().await;
        let limit = limit.clamp(1, 200);
        let mut items: Vec<ModerationEntry> = g
            .banned_users
            .iter()
            .map(|(id, s)| ModerationEntry {
                user_id: UserId(*id),
                reason: s.reason.clone(),
                created_at: s.created_at,
            })
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(limit);
        items
    }

    /// Muted users newest-first (limit clamped 1..=200).
    pub async fn list_muted(&self, limit: usize) -> Vec<ModerationEntry> {
        let g = self.inner.lock().await;
        let limit = limit.clamp(1, 200);
        let mut items: Vec<ModerationEntry> = g
            .muted_users
            .iter()
            .map(|(id, s)| ModerationEntry {
                user_id: UserId(*id),
                reason: s.reason.clone(),
                created_at: s.created_at,
            })
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(limit);
        items
    }

    /// Lookup ban/mute status for one user (ops console).
    pub async fn user_status(&self, user_id: UserId) -> UserModerationStatus {
        let g = self.inner.lock().await;
        let ban = g.banned_users.get(&user_id.0);
        let mute = g.muted_users.get(&user_id.0);
        UserModerationStatus {
            user_id,
            banned: ban.is_some(),
            muted: mute.is_some(),
            ban_reason: ban.map(|s| s.reason.clone()),
            mute_reason: mute.map(|s| s.reason.clone()),
            banned_at: ban.map(|s| s.created_at),
            muted_at: mute.map(|s| s.created_at),
        }
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

// ── Sensitive-word filter (content safety v1 hook) ───────────────────────────

/// Case-insensitive substring blocklist for chat bodies.
///
/// Empty by default (open). Configure via `CHAT_BLOCKLIST` (comma-separated) or
/// construct in tests with [`WordFilter::from_words`]. Does not claim full NLP —
/// P2 policy v1 substring gate only.
#[derive(Debug, Clone, Default)]
pub struct WordFilter {
    /// Lowercased terms.
    terms: Vec<String>,
}

impl WordFilter {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_words<I, S>(words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let terms = words
            .into_iter()
            .map(|s| s.as_ref().trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        Self { terms }
    }

    /// `CHAT_BLOCKLIST=foo,bar,baz`
    pub fn from_env() -> Self {
        let raw = std::env::var("CHAT_BLOCKLIST").unwrap_or_default();
        if raw.trim().is_empty() {
            return Self::empty();
        }
        Self::from_words(raw.split(','))
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn term_count(&self) -> usize {
        self.terms.len()
    }

    /// Returns `Err(ForbiddenPolicy)` when body contains a blocked term.
    pub fn check(&self, body: &str) -> Result<(), AppError> {
        if self.terms.is_empty() {
            return Ok(());
        }
        let lower = body.to_ascii_lowercase();
        for term in &self.terms {
            if lower.contains(term) {
                return Err(AppError::new(
                    ErrorCode::ForbiddenPolicy,
                    "message blocked by content policy",
                ));
            }
        }
        Ok(())
    }
}

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
        let status = m.user_status(user).await;
        assert!(status.banned);
        assert_eq!(status.ban_reason.as_deref(), Some("spam"));
        let banned = m.list_banned(10).await;
        assert_eq!(banned.len(), 1);
        assert_eq!(banned[0].user_id, user);
        m.unban_user(admin, user, "appeal").await.unwrap();
        assert!(!m.is_banned(user).await);
        let audit = m.recent_audit(10).await;
        assert_eq!(audit[0].action, "unban_user");
        assert_eq!(audit[1].action, "ban_user");
    }

    #[tokio::test]
    async fn mute_requires_admin_and_unmutes() {
        let m = MemoryModeration::new();
        let admin = UserId::new();
        let user = UserId::new();

        let err = m.mute_user(admin, user, "noise").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
        assert!(!m.is_muted(user).await);

        m.grant_admin(admin).await;
        m.mute_user(admin, user, "spam chat").await.unwrap();
        assert!(m.is_muted(user).await);

        let audit = m.recent_audit(10).await;
        assert_eq!(audit[0].action, "mute_user");
        assert_eq!(audit[0].detail, "spam chat");

        m.unmute_user(admin, user, "appeal accepted").await.unwrap();
        assert!(!m.is_muted(user).await);
        let audit = m.recent_audit(10).await;
        assert_eq!(audit[0].action, "unmute_user");
    }

    #[tokio::test]
    async fn unmute_requires_admin() {
        let m = MemoryModeration::new();
        let admin = UserId::new();
        let other = UserId::new();
        let user = UserId::new();
        m.grant_admin(admin).await;
        m.mute_user(admin, user, "x").await.unwrap();
        let err = m.unmute_user(other, user, "nope").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
        assert!(m.is_muted(user).await);
    }

    #[tokio::test]
    async fn role_rank_moderator_below_admin() {
        let m = MemoryModeration::new();
        let mod_user = UserId::new();
        let target = UserId::new();
        m.grant_role(mod_user, AdminRole::Moderator).await;
        assert!(m.is_admin(mod_user).await);
        assert_eq!(m.admin_role(mod_user).await, Some(AdminRole::Moderator));
        // Moderator meets moderator threshold.
        m.require_role(mod_user, AdminRole::Moderator)
            .await
            .unwrap();
        // Moderator does not meet full admin.
        let err = m
            .require_role(mod_user, AdminRole::Admin)
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
        // Ban path still requires full admin via require_admin.
        let err = m.ban_user(mod_user, target, "x").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[test]
    fn admin_role_parse_and_rank() {
        assert_eq!(AdminRole::parse("mod"), Some(AdminRole::Moderator));
        assert_eq!(AdminRole::parse("ops"), Some(AdminRole::Ops));
        assert!(AdminRole::Admin.meets(AdminRole::Ops));
        assert!(!AdminRole::Ops.meets(AdminRole::Moderator));
    }

    #[test]
    fn word_filter_blocks_substring_case_insensitive() {
        let f = WordFilter::from_words(["spam", "scam"]);
        assert!(f.check("hello world").is_ok());
        let err = f.check("Buy SPAMware now").unwrap_err();
        assert_eq!(err.code, ErrorCode::ForbiddenPolicy);
        assert!(f.check("nice SCAM link").is_err());
        assert!(WordFilter::empty().check("anything spam").is_ok());
    }
}
