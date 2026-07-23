//! OAuth exchange scaffold (WBS E2.1).
//!
//! Real Google/Apple token verification needs vendor client IDs + JWKS. This
//! module provides:
//!
//! 1. **Stub mode** (`OAUTH_STUB=1` or local `APP_ENV`) — accept
//!    `id_token = "stub:<email>"` and mint a normal session. Dogfood only.
//! 2. **Configured mode** — require `OAUTH_PROVIDERS` + per-provider audience;
//!    currently still rejects real tokens with a clear error until JWKS is
//!    wired (scaffold, not a silent accept).
//!
//! Production forbids stub mode via [`crate::guards`].

use anylive_common::{AppError, ErrorCode};

/// Supported OAuth providers (lowercase).
pub const OAUTH_PROVIDERS: &[&str] = &["google", "apple"];

#[derive(Debug, Clone)]
pub struct OauthConfig {
    /// When true, `id_token` of form `stub:user@example.com` is accepted.
    pub stub_enabled: bool,
    /// Comma-list of enabled providers (empty = all known, for local).
    pub enabled: Vec<String>,
}

impl Default for OauthConfig {
    fn default() -> Self {
        Self {
            stub_enabled: true,
            enabled: OAUTH_PROVIDERS.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

impl OauthConfig {
    pub fn from_env() -> Self {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into());
        let is_prod = crate::guards::is_production_env(&app_env);
        let is_local = crate::guards::is_local_env(&app_env);
        let stub_flag = std::env::var("OAUTH_STUB")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);
        // Stub only when explicitly enabled, or local default without production.
        let stub_enabled = if is_prod {
            false
        } else {
            stub_flag || is_local
        };
        let enabled = std::env::var("OAUTH_PROVIDERS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_ascii_lowercase())
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                OAUTH_PROVIDERS.iter().map(|s| (*s).to_string()).collect()
            });
        Self {
            stub_enabled,
            enabled,
        }
    }

    pub fn provider_enabled(&self, provider: &str) -> bool {
        let p = provider.trim().to_ascii_lowercase();
        self.enabled.iter().any(|e| e == &p)
    }
}

/// Resolve an OAuth assertion into a normalized email.
///
/// Stub tokens: `stub:user@example.com` (case-insensitive provider list).
/// Real tokens: currently return a clear "not implemented" so misconfig is loud.
pub fn resolve_oauth_email(
    cfg: &OauthConfig,
    provider: &str,
    id_token: &str,
) -> Result<String, AppError> {
    let provider = provider.trim().to_ascii_lowercase();
    if !OAUTH_PROVIDERS.contains(&provider.as_str()) {
        return Err(AppError::validation(format!(
            "unsupported oauth provider: {provider}"
        )));
    }
    if !cfg.provider_enabled(&provider) {
        return Err(AppError::validation(format!(
            "oauth provider disabled: {provider}"
        )));
    }
    let token = id_token.trim();
    if token.is_empty() {
        return Err(AppError::validation("id_token must not be empty"));
    }

    if let Some(email) = token.strip_prefix("stub:") {
        if !cfg.stub_enabled {
            return Err(AppError::new(
                ErrorCode::Forbidden,
                "oauth stub tokens are disabled",
            ));
        }
        let email = email.trim().to_ascii_lowercase();
        if !email.contains('@') || email.len() < 5 {
            return Err(AppError::validation(
                "stub id_token must be stub:<email>",
            ));
        }
        return Ok(email);
    }

    // Real provider verification is intentionally not silent-accept.
    Err(AppError::new(
        ErrorCode::Validation,
        format!(
            "oauth provider {provider}: real id_token verification not configured \
             (use stub:<email> when OAUTH_STUB=1 / local, or wire JWKS)"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_resolves_email() {
        let cfg = OauthConfig {
            stub_enabled: true,
            enabled: vec!["google".into(), "apple".into()],
        };
        let e = resolve_oauth_email(&cfg, "google", "stub:Ada@Example.com").unwrap();
        assert_eq!(e, "ada@example.com");
    }

    #[test]
    fn stub_disabled_rejects() {
        let cfg = OauthConfig {
            stub_enabled: false,
            enabled: vec!["google".into()],
        };
        let err = resolve_oauth_email(&cfg, "google", "stub:a@b.com").unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[test]
    fn real_token_not_silent() {
        let cfg = OauthConfig::default();
        let err = resolve_oauth_email(&cfg, "apple", "eyJhbGciOi...").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn unknown_provider() {
        let cfg = OauthConfig::default();
        let err = resolve_oauth_email(&cfg, "facebook", "stub:a@b.com").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }
}
