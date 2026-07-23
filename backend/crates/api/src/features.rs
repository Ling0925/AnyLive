//! Process-level feature flags for soft launch / GA kill-switches (P5).
//!
//! **P1 exit discipline (docs/product/06):** `FEATURE_PK` / `FEATURE_COHOST`
//! default **OFF** when unset so P3 is not a dogfood exit criterion.
//! Other flags remain open for local/dev unless overridden.
//!
//! Env:
//! - `FEATURE_PUBLIC_REGISTER=0` — force invite-only style registration (pairs with INVITE_ONLY)
//! - `FEATURE_REAL_PAY=0` — refuse non-mock pay channel order creation
//! - `FEATURE_PK=1` — enable PK start (default **off**)
//! - `FEATURE_COHOST=1` — enable co-host invites (default **off**)
//! - `FEATURE_CLIENT_EVENTS=0` — drop analytics ingest

use anylive_common::{AppError, ErrorCode};

#[derive(Debug, Clone)]
pub struct FeatureFlags {
    pub public_register: bool,
    pub real_pay: bool,
    pub pk: bool,
    pub cohost: bool,
    pub client_events: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        // Match from_env() P1-safe defaults: PK/cohost off; rest open for local.
        Self {
            public_register: true,
            real_pay: true,
            pk: false,
            cohost: false,
            client_events: true,
        }
    }
}

impl FeatureFlags {
    pub fn all_enabled() -> Self {
        Self {
            public_register: true,
            real_pay: true,
            pk: true,
            cohost: true,
            client_events: true,
        }
    }

    pub fn from_env() -> Self {
        Self {
            public_register: env_flag("FEATURE_PUBLIC_REGISTER", true),
            real_pay: env_flag("FEATURE_REAL_PAY", true),
            // P3 surfaces: default OFF until explicitly enabled (plan 06 §8.1).
            pk: env_flag("FEATURE_PK", false),
            cohost: env_flag("FEATURE_COHOST", false),
            client_events: env_flag("FEATURE_CLIENT_EVENTS", true),
        }
    }

    pub fn require_pk(&self) -> Result<(), AppError> {
        if self.pk {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "PK is disabled by feature flag",
            ))
        }
    }

    pub fn require_cohost(&self) -> Result<(), AppError> {
        if self.cohost {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "co-host is disabled by feature flag",
            ))
        }
    }

    pub fn require_client_events(&self) -> Result<(), AppError> {
        if self.client_events {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "client events ingest is disabled by feature flag",
            ))
        }
    }

    pub fn require_real_pay(&self) -> Result<(), AppError> {
        if self.real_pay {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "real pay channels disabled by feature flag (mock only)",
            ))
        }
    }
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim().to_ascii_lowercase();
            match t.as_str() {
                "1" | "true" | "yes" | "on" | "enable" | "enabled" => true,
                "0" | "false" | "no" | "off" | "disable" | "disabled" => false,
                _ => default,
            }
        }
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_pk_when_disabled() {
        let mut f = FeatureFlags::all_enabled();
        f.pk = false;
        assert_eq!(f.require_pk().unwrap_err().code, ErrorCode::ForbiddenPolicy);
    }

    #[test]
    fn default_disables_p3_surfaces() {
        let f = FeatureFlags::default();
        assert!(!f.pk);
        assert!(!f.cohost);
        assert!(f.public_register);
        assert!(f.client_events);
    }
}
