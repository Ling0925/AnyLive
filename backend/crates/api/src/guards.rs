//! Startup / production safety guards (pure functions, unit-tested).
//!
//! When `APP_ENV` is `production` or `prod`, reject insecure defaults so a
//! misconfigured deploy fails closed at process start rather than at first
//! request.

/// Default JWT access secret used by [`anylive_auth::JwtConfig::default`].
pub const DEFAULT_JWT_ACCESS_SECRET: &str = "dev-access-secret-change-me-32chars!!";
/// Default JWT refresh secret used by [`anylive_auth::JwtConfig::default`].
pub const DEFAULT_JWT_REFRESH_SECRET: &str = "dev-refresh-secret-change-me-32chars!";
/// Default Centrifugo HMAC secret used by [`anylive_realtime::CentrifugoConfig::default`].
pub const DEFAULT_CENTRIFUGO_TOKEN_SECRET: &str = "anylive-dev-token-secret-change-me";

/// True when `app_env` denotes a production deployment (`production` / `prod`, case-insensitive).
pub fn is_production_env(app_env: &str) -> bool {
    matches!(
        app_env.trim().to_ascii_lowercase().as_str(),
        "production" | "prod"
    )
}

/// Reject default / weak JWT secrets in production.
pub fn check_jwt_secrets_for_production(
    access_secret: &str,
    refresh_secret: &str,
) -> Result<(), String> {
    if access_secret == DEFAULT_JWT_ACCESS_SECRET || refresh_secret == DEFAULT_JWT_REFRESH_SECRET {
        return Err("production forbids default JWT secrets".into());
    }
    if access_secret.len() < 32 || refresh_secret.len() < 32 {
        return Err("production JWT secrets must be >= 32 bytes".into());
    }
    if access_secret == refresh_secret {
        return Err("production access/refresh secrets must differ".into());
    }
    Ok(())
}

/// Reject fixed/dev OTP mode in production (`OtpConfig::dev`).
pub fn check_otp_for_production(dev_fixed_otp: bool) -> Result<(), String> {
    if dev_fixed_otp {
        return Err("production forbids fixed OTP mode (OtpConfig::dev)".into());
    }
    Ok(())
}

/// When realtime is enabled, require a non-default Centrifugo token secret.
pub fn check_centrifugo_for_production(
    token_secret: &str,
    realtime_used: bool,
) -> Result<(), String> {
    if !realtime_used {
        return Ok(());
    }
    if token_secret == DEFAULT_CENTRIFUGO_TOKEN_SECRET {
        return Err(
            "production forbids default CENTRIFUGO_TOKEN_SECRET when realtime is used".into(),
        );
    }
    if token_secret.len() < 16 {
        return Err("production CENTRIFUGO_TOKEN_SECRET must be >= 16 bytes".into());
    }
    Ok(())
}

/// Reject open SRS webhook auth in production (secret must be non-empty).
pub fn check_srs_webhook_for_production(webhook_secret: Option<&str>) -> Result<(), String> {
    match webhook_secret {
        Some(s) if !s.trim().is_empty() => Ok(()),
        _ => Err(
            "production requires SRS_WEBHOOK_SECRET (open webhooks forbidden)".into(),
        ),
    }
}

/// Composite production guard used at process startup / `AppState::from_env`.
///
/// No-op when `app_env` is not production. `centrifugo_token_secret` is checked
/// only when `realtime_used` is true (pass `None` to skip).
///
/// `srs_webhook_secret` is the raw `SRS_WEBHOOK_SECRET` env value (or `None` if
/// unset). Production requires a non-empty secret so publish hooks cannot be
/// forged without credentials.
pub fn check_production_secrets(
    app_env: &str,
    access_secret: &str,
    refresh_secret: &str,
    dev_fixed_otp: bool,
    centrifugo_token_secret: Option<&str>,
    realtime_used: bool,
    srs_webhook_secret: Option<&str>,
) -> Result<(), String> {
    if !is_production_env(app_env) {
        return Ok(());
    }
    check_jwt_secrets_for_production(access_secret, refresh_secret)?;
    check_otp_for_production(dev_fixed_otp)?;
    if let Some(secret) = centrifugo_token_secret {
        check_centrifugo_for_production(secret, realtime_used)?;
    } else if realtime_used {
        return Err(
            "production requires CENTRIFUGO_TOKEN_SECRET when realtime is used".into(),
        );
    }
    check_srs_webhook_for_production(srs_webhook_secret)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_production_env_matches_prod_aliases() {
        assert!(is_production_env("production"));
        assert!(is_production_env("PRODUCTION"));
        assert!(is_production_env("prod"));
        assert!(is_production_env(" Prod "));
        assert!(!is_production_env("local"));
        assert!(!is_production_env("development"));
        assert!(!is_production_env("staging"));
        assert!(!is_production_env(""));
    }

    #[test]
    fn dev_allows_defaults() {
        assert!(check_production_secrets(
            "local",
            DEFAULT_JWT_ACCESS_SECRET,
            DEFAULT_JWT_REFRESH_SECRET,
            true,
            Some(DEFAULT_CENTRIFUGO_TOKEN_SECRET),
            true,
            None,
        )
        .is_ok());
    }

    #[test]
    fn prod_rejects_default_jwt_secrets() {
        let err = check_production_secrets(
            "production",
            DEFAULT_JWT_ACCESS_SECRET,
            DEFAULT_JWT_REFRESH_SECRET,
            false,
            Some("strong-centrifugo-secret-xyz"),
            true,
            Some("webhook-secret"),
        )
        .unwrap_err();
        assert!(err.contains("default JWT"));
    }

    #[test]
    fn prod_rejects_partial_default_access_only() {
        let err = check_jwt_secrets_for_production(
            DEFAULT_JWT_ACCESS_SECRET,
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap_err();
        assert!(err.contains("default JWT"));
    }

    #[test]
    fn prod_rejects_short_or_shared_jwt() {
        assert!(check_jwt_secrets_for_production("short", "also-short").is_err());
        let shared = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert!(check_jwt_secrets_for_production(shared, shared)
            .unwrap_err()
            .contains("differ"));
    }

    #[test]
    fn prod_rejects_fixed_otp() {
        let err = check_production_secrets(
            "prod",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            true,
            Some("strong-centrifugo-secret-xyz"),
            true,
            Some("webhook-secret"),
        )
        .unwrap_err();
        assert!(err.contains("fixed OTP"));
    }

    #[test]
    fn prod_rejects_default_centrifugo_when_realtime_used() {
        let err = check_production_secrets(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some(DEFAULT_CENTRIFUGO_TOKEN_SECRET),
            true,
            Some("webhook-secret"),
        )
        .unwrap_err();
        assert!(err.contains("CENTRIFUGO_TOKEN_SECRET"));
    }

    #[test]
    fn prod_allows_default_centrifugo_when_realtime_unused() {
        assert!(check_production_secrets(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some(DEFAULT_CENTRIFUGO_TOKEN_SECRET),
            false,
            Some("webhook-secret"),
        )
        .is_ok());
    }

    #[test]
    fn prod_requires_centrifugo_secret_present_when_realtime_used() {
        let err = check_production_secrets(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            None,
            true,
            Some("webhook-secret"),
        )
        .unwrap_err();
        assert!(err.contains("CENTRIFUGO_TOKEN_SECRET"));
    }

    #[test]
    fn prod_requires_srs_webhook_secret() {
        let err = check_production_secrets(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some("production-centrifugo-hmac-key"),
            true,
            None,
        )
        .unwrap_err();
        assert!(err.contains("SRS_WEBHOOK_SECRET"));

        let err_empty = check_production_secrets(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some("production-centrifugo-hmac-key"),
            true,
            Some("  "),
        )
        .unwrap_err();
        assert!(err_empty.contains("SRS_WEBHOOK_SECRET"));
    }

    #[test]
    fn prod_ok_with_strong_secrets() {
        assert!(check_production_secrets(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some("production-centrifugo-hmac-key"),
            true,
            Some("production-srs-webhook-secret"),
        )
        .is_ok());
    }

    #[test]
    fn pure_otp_and_centrifugo_helpers() {
        assert!(check_otp_for_production(false).is_ok());
        assert!(check_otp_for_production(true).is_err());
        assert!(check_centrifugo_for_production("ok-secret-16chars", true).is_ok());
        assert!(check_centrifugo_for_production(DEFAULT_CENTRIFUGO_TOKEN_SECRET, true).is_err());
        assert!(check_centrifugo_for_production(DEFAULT_CENTRIFUGO_TOKEN_SECRET, false).is_ok());
        assert!(check_centrifugo_for_production("short", true).is_err());
        assert!(check_srs_webhook_for_production(Some("secret")).is_ok());
        assert!(check_srs_webhook_for_production(None).is_err());
        assert!(check_srs_webhook_for_production(Some("")).is_err());
    }
}
