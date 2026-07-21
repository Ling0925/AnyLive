//! CORS layer construction for the HTTP API.
//!
//! - Non-production: [`CorsLayer::permissive`] (local H5/admin/vite origins vary).
//! - Production: require `CORS_ALLOWED_ORIGINS` (comma-separated absolute origins).
//!   Fail closed if missing or empty when `APP_ENV` is production/prod.

use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::guards::is_production_env;

/// Build a CORS layer from environment.
///
/// Reads:
/// - `APP_ENV` — production aliases force restricted CORS
/// - `CORS_ALLOWED_ORIGINS` — comma-separated list, e.g. `https://app.example,https://admin.example`
pub fn cors_layer_from_env() -> Result<CorsLayer, String> {
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into());
    cors_layer_for_env(&app_env, std::env::var("CORS_ALLOWED_ORIGINS").ok().as_deref())
}

/// Pure helper for tests: choose CORS policy from env strings.
pub fn cors_layer_for_env(
    app_env: &str,
    allowed_origins: Option<&str>,
) -> Result<CorsLayer, String> {
    if !is_production_env(app_env) {
        return Ok(CorsLayer::permissive());
    }
    let raw = allowed_origins
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "production requires CORS_ALLOWED_ORIGINS (comma-separated absolute origins)".to_string()
        })?;
    let origins: Vec<_> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<axum::http::HeaderValue>().map_err(|e| {
                format!("invalid CORS origin `{s}`: {e}")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if origins.is_empty() {
        return Err(
            "production requires CORS_ALLOWED_ORIGINS (comma-separated absolute origins)".into(),
        );
    }
    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any))
}

/// True when the given policy would use permissive CORS (non-prod).
pub fn is_permissive_cors(app_env: &str) -> bool {
    !is_production_env(app_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_is_permissive() {
        assert!(is_permissive_cors("local"));
        assert!(is_permissive_cors("development"));
        let layer = cors_layer_for_env("local", None).unwrap();
        // CorsLayer is opaque; just ensure construction succeeds.
        let _ = layer;
    }

    #[test]
    fn prod_requires_origins() {
        let err = cors_layer_for_env("production", None).unwrap_err();
        assert!(err.contains("CORS_ALLOWED_ORIGINS"));
        let err = cors_layer_for_env("prod", Some("  ")).unwrap_err();
        assert!(err.contains("CORS_ALLOWED_ORIGINS"));
    }

    #[test]
    fn prod_accepts_comma_list() {
        let layer = cors_layer_for_env(
            "production",
            Some("https://app.example.com, https://admin.example.com"),
        )
        .unwrap();
        let _ = layer;
    }

    #[test]
    fn prod_rejects_bad_origin_bytes() {
        // HeaderValue rejects ASCII control characters inside a token.
        let err =
            cors_layer_for_env("production", Some("https://ok.example,https://bad\x01.example"))
                .unwrap_err();
        assert!(
            err.contains("invalid CORS origin") || err.contains("CORS_ALLOWED_ORIGINS"),
            "unexpected err: {err}"
        );
    }
}
