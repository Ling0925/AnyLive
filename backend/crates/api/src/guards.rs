//! Startup / production safety guards (pure functions, unit-tested).
//!
//! When `APP_ENV` is `production` or `prod`, reject insecure defaults so a
//! misconfigured deploy fails closed at process start rather than at first
//! request. Feature flags (`ALLOW_DEV_OTP`, `ALLOW_MOCK_TOPUP`, …) are also
//! validated here when production.

use anylive_auth::env_flag_enabled;

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

/// True when `app_env` is explicitly local/dev/test (or empty).
pub fn is_local_env(app_env: &str) -> bool {
    matches!(
        app_env.trim().to_ascii_lowercase().as_str(),
        "" | "local" | "development" | "dev" | "test"
    )
}

/// Mock topup is allowed only with an explicit opt-in flag — never merely
/// because `APP_ENV` is not production.
pub fn mock_topup_allowed() -> bool {
    env_flag_enabled("ALLOW_MOCK_TOPUP")
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

/// Reject fixed/dev OTP mode in production (`OtpConfig::dev` / `ALLOW_DEV_OTP`).
pub fn check_otp_for_production(dev_fixed_otp: bool) -> Result<(), String> {
    if dev_fixed_otp {
        return Err("production forbids fixed OTP mode (OtpConfig::dev / ALLOW_DEV_OTP)".into());
    }
    Ok(())
}

/// Production must configure a real OTP notifier with a delivery endpoint.
///
/// - `smtp` / `http` / `webhook` require `OTP_HTTP_URL` or `OTP_SMTP_WEBHOOK_URL`
/// - `log` / `noop` / empty are forbidden in production
pub fn check_otp_notifier_for_production(notifier_kind: &str) -> Result<(), String> {
    match notifier_kind.trim().to_ascii_lowercase().as_str() {
        "smtp" | "http" | "webhook" => {
            let url = std::env::var("OTP_HTTP_URL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    std::env::var("OTP_SMTP_WEBHOOK_URL")
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                });
            let Some(url) = url else {
                return Err(
                    "production OTP_NOTIFIER=smtp|http|webhook requires OTP_HTTP_URL or OTP_SMTP_WEBHOOK_URL"
                        .into(),
                );
            };
            if !(url.starts_with("https://") || url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")) {
                return Err(
                    "production OTP_HTTP_URL must be https:// (or localhost for emergency break-glass)"
                        .into(),
                );
            }
            Ok(())
        }
        "log" | "console" => Err(
            "production forbids OTP_NOTIFIER=log (no real delivery); use smtp|http with URL".into(),
        ),
        "noop" | "" | "none" | "unconfigured" => Err(
            "production requires OTP_NOTIFIER=smtp|http with OTP_HTTP_URL; noop/unconfigured forbidden"
                .into(),
        ),
        other => Err(format!(
            "production OTP_NOTIFIER={other} is not a known delivery backend"
        )),
    }
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

/// Reject mock topup + insecure JWT flags in production.
pub fn check_feature_flags_for_production() -> Result<(), String> {
    if env_flag_enabled("ALLOW_MOCK_TOPUP") {
        return Err("production forbids ALLOW_MOCK_TOPUP".into());
    }
    if env_flag_enabled("ALLOW_DEV_OTP") {
        return Err("production forbids ALLOW_DEV_OTP".into());
    }
    if env_flag_enabled("ALLOW_INSECURE_JWT") {
        return Err("production forbids ALLOW_INSECURE_JWT".into());
    }
    if env_flag_enabled("PAY_ENABLE_MOCK") {
        return Err("production forbids PAY_ENABLE_MOCK".into());
    }
    if env_flag_enabled("OAUTH_STUB") {
        return Err("production forbids OAUTH_STUB".into());
    }
    if anylive_pay::mock_pay_enabled_from_env() {
        return Err(
            "production forbids mock pay channel (PAY_CHANNELS=mock / PAY_ENABLE_MOCK / ALLOW_MOCK_TOPUP)"
                .into(),
        );
    }
    Ok(())
}

/// Composite production guard used at process startup / `AppState::from_env`.
///
/// No-op when `app_env` is not production. `centrifugo_token_secret` is checked
/// only when `realtime_used` is true (pass `None` to skip).
///
/// `srs_webhook_secret` is the raw `SRS_WEBHOOK_SECRET` env value (or `None` if
/// unset). Production requires a non-empty secret so publish hooks cannot be
/// forged without credentials.
///
/// `otp_notifier_kind` is the raw `OTP_NOTIFIER` env value (defaults empty).
pub fn check_production_secrets(
    app_env: &str,
    access_secret: &str,
    refresh_secret: &str,
    dev_fixed_otp: bool,
    centrifugo_token_secret: Option<&str>,
    realtime_used: bool,
    srs_webhook_secret: Option<&str>,
) -> Result<(), String> {
    check_production_secrets_ext(
        app_env,
        access_secret,
        refresh_secret,
        dev_fixed_otp,
        centrifugo_token_secret,
        realtime_used,
        srs_webhook_secret,
        None,
    )
}

/// Extended production guard including OTP notifier kind.
#[allow(clippy::too_many_arguments)]
pub fn check_production_secrets_ext(
    app_env: &str,
    access_secret: &str,
    refresh_secret: &str,
    dev_fixed_otp: bool,
    centrifugo_token_secret: Option<&str>,
    realtime_used: bool,
    srs_webhook_secret: Option<&str>,
    otp_notifier_kind: Option<&str>,
) -> Result<(), String> {
    if !is_production_env(app_env) {
        return Ok(());
    }
    check_jwt_secrets_for_production(access_secret, refresh_secret)?;
    check_otp_for_production(dev_fixed_otp)?;
    check_feature_flags_for_production()?;
    if let Some(kind) = otp_notifier_kind {
        check_otp_notifier_for_production(kind)?;
    }
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

/// Constant-time equality for secrets of equal length (timing-safe compare).
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    if a.len() != b.len() {
        return false;
    }
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Env mutations are process-global; serialize flag tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
    fn mock_topup_requires_explicit_flag() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        assert!(!mock_topup_allowed());
        std::env::set_var("ALLOW_MOCK_TOPUP", "1");
        assert!(mock_topup_allowed());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
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
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        std::env::remove_var("ALLOW_DEV_OTP");
        std::env::remove_var("ALLOW_INSECURE_JWT");
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
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        std::env::remove_var("ALLOW_DEV_OTP");
        std::env::remove_var("ALLOW_INSECURE_JWT");
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
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        std::env::remove_var("ALLOW_DEV_OTP");
        std::env::remove_var("ALLOW_INSECURE_JWT");
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
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        std::env::remove_var("ALLOW_DEV_OTP");
        std::env::remove_var("ALLOW_INSECURE_JWT");
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
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        std::env::remove_var("ALLOW_DEV_OTP");
        std::env::remove_var("ALLOW_INSECURE_JWT");
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
    fn prod_requires_otp_notifier() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_MOCK_TOPUP");
        std::env::remove_var("ALLOW_DEV_OTP");
        std::env::remove_var("ALLOW_INSECURE_JWT");
        std::env::remove_var("OTP_HTTP_URL");
        std::env::remove_var("OTP_SMTP_WEBHOOK_URL");
        let err = check_production_secrets_ext(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some("production-centrifugo-hmac-key"),
            true,
            Some("production-srs-webhook-secret"),
            Some(""),
        )
        .unwrap_err();
        assert!(err.contains("OTP_NOTIFIER"));

        // log has no real delivery — forbidden in production.
        let err_log = check_production_secrets_ext(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some("production-centrifugo-hmac-key"),
            true,
            Some("production-srs-webhook-secret"),
            Some("log"),
        )
        .unwrap_err();
        assert!(err_log.contains("log") || err_log.contains("OTP_NOTIFIER"));

        // smtp without URL fails.
        let err_smtp = check_otp_notifier_for_production("smtp").unwrap_err();
        assert!(err_smtp.contains("OTP_HTTP_URL") || err_smtp.contains("URL"));

        std::env::set_var("OTP_HTTP_URL", "https://mail.example/send");
        assert!(check_otp_notifier_for_production("http").is_ok());
        assert!(check_production_secrets_ext(
            "production",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            Some("production-centrifugo-hmac-key"),
            true,
            Some("production-srs-webhook-secret"),
            Some("http"),
        )
        .is_ok());
        std::env::remove_var("OTP_HTTP_URL");
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

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("ab", "abc"));
    }
}
