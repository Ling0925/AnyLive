//! Soft-launch invite / email whitelist gate (P2 M2.4).
//!
//! When `INVITE_ONLY=1` (or `true`), OTP verify requires the email to appear in
//! `INVITE_EMAIL_ALLOWLIST` (comma-separated, case-insensitive) **or** the request
//! body / header to carry a valid `INVITE_CODES` token (comma-separated).
//!
//! Default (unset): open registration for local/dogfood.

use std::collections::HashSet;

use anylive_common::{AppError, ErrorCode};

/// Runtime invite policy loaded from env at process start.
#[derive(Debug, Clone)]
pub struct InviteGate {
    /// When false, all emails pass.
    pub enabled: bool,
    allowlist: HashSet<String>,
    codes: HashSet<String>,
}

impl Default for InviteGate {
    fn default() -> Self {
        Self {
            enabled: false,
            allowlist: HashSet::new(),
            codes: HashSet::new(),
        }
    }
}

impl InviteGate {
    pub fn open() -> Self {
        Self::default()
    }

    /// Parse env:
    /// - `INVITE_ONLY=1|true` enables the gate
    /// - `INVITE_EMAIL_ALLOWLIST=a@x.com,b@y.com`
    /// - `INVITE_CODES=code1,code2`
    pub fn from_env() -> Self {
        let enabled = matches!(
            std::env::var("INVITE_ONLY").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
        );
        if !enabled {
            return Self::open();
        }
        let allowlist = std::env::var("INVITE_EMAIL_ALLOWLIST")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect();
        let codes = std::env::var("INVITE_CODES")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self {
            enabled: true,
            allowlist,
            codes,
        }
    }

    /// Test helper with fixed allowlist + codes.
    pub fn restricted(emails: &[&str], codes: &[&str]) -> Self {
        Self {
            enabled: true,
            allowlist: emails.iter().map(|e| e.to_ascii_lowercase()).collect(),
            codes: codes.iter().map(|c| (*c).to_string()).collect(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check whether `email` may complete OTP verify, optionally with `invite_code`.
    pub fn check(&self, email: &str, invite_code: Option<&str>) -> Result<(), AppError> {
        if !self.enabled {
            return Ok(());
        }
        let email_key = email.trim().to_ascii_lowercase();
        if self.allowlist.contains(&email_key) {
            return Ok(());
        }
        if let Some(code) = invite_code.map(str::trim).filter(|c| !c.is_empty()) {
            if self.codes.contains(code) {
                return Ok(());
            }
        }
        Err(AppError::new(
            ErrorCode::ForbiddenPolicy,
            "invite required: email not on allowlist and invite code missing/invalid",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_allows_anyone() {
        let g = InviteGate::open();
        assert!(g.check("anyone@example.com", None).is_ok());
    }

    #[test]
    fn restricted_allows_allowlist_and_code() {
        let g = InviteGate::restricted(&["host@example.com"], &["VIP-1"]);
        assert!(g.check("host@example.com", None).is_ok());
        assert!(g.check("HOST@example.com", None).is_ok());
        assert!(g.check("other@example.com", Some("VIP-1")).is_ok());
        let err = g.check("other@example.com", None).unwrap_err();
        assert_eq!(err.code, ErrorCode::ForbiddenPolicy);
        let err = g.check("other@example.com", Some("wrong")).unwrap_err();
        assert_eq!(err.code, ErrorCode::ForbiddenPolicy);
    }
}
