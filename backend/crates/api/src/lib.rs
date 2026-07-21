//! HTTP routes and OpenAPI surface for AnyLive API.

mod auth_user;
mod error;
mod routes;
mod state;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub use auth_user::AuthUser;
pub use error::ApiError;
pub use state::AppState;

/// Build the Axum router with in-memory auth (binary + integration tests).
pub fn build_app() -> Router {
    build_app_with_state(AppState::dev())
}

/// Build router with explicit shared state.
pub fn build_app_with_state(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/ready", get(routes::ready))
        .route("/api/v1/meta", get(routes::meta))
        .route("/api/v1/auth/otp/send", post(routes::otp_send))
        .route("/api/v1/auth/otp/verify", post(routes::otp_verify))
        .route("/api/v1/auth/token/refresh", post(routes::token_refresh))
        .route("/api/v1/auth/logout", post(routes::logout))
        .route("/api/v1/me", get(routes::me))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health,
        routes::ready,
        routes::meta,
        routes::otp_send,
        routes::otp_verify,
        routes::token_refresh,
        routes::logout,
        routes::me,
    ),
    components(schemas(
        routes::HealthResponse,
        routes::MetaResponse,
        routes::OtpSendBody,
        routes::OtpVerifyBody,
        routes::RefreshBody,
        routes::LogoutBody,
        routes::TokenPairDto,
        routes::UserDto,
        routes::AuthSessionResponse,
    )),
    tags(
        (name = "system", description = "Health and metadata"),
        (name = "auth", description = "Authentication"),
        (name = "users", description = "User profile")
    ),
    modifiers(&SecurityAddon),
    info(title = "AnyLive API", version = "0.1.0")
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}

/// Serialize OpenAPI JSON (for contract export tests).
pub fn openapi_json() -> serde_json::Value {
    serde_json::to_value(ApiDoc::openapi()).expect("openapi serializable")
}

#[derive(Debug, Serialize)]
pub struct ServiceInfo {
    pub name: &'static str,
    pub version: &'static str,
}

pub fn service_info() -> ServiceInfo {
    ServiceInfo {
        name: "anylive-api",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(res: axum::response::Response) -> serde_json::Value {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        if bytes.is_empty() {
            return serde_json::Value::Null;
        }
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = build_app();
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "anylive-api");
    }

    #[tokio::test]
    async fn ready_returns_ok() {
        let app = build_app();
        let res = app
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["ready"], true);
    }

    #[tokio::test]
    async fn meta_includes_version() {
        let app = build_app();
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/meta")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["name"], "anylive-api");
        assert!(!json["version"].as_str().unwrap().is_empty());
    }

    #[test]
    fn openapi_contains_auth_paths() {
        let doc = openapi_json();
        let paths = doc.get("paths").unwrap();
        assert!(paths.get("/health").is_some());
        assert!(paths.get("/ready").is_some());
        assert!(paths.get("/api/v1/meta").is_some());
        assert!(paths.get("/api/v1/auth/otp/send").is_some());
        assert!(paths.get("/api/v1/auth/otp/verify").is_some());
        assert!(paths.get("/api/v1/auth/token/refresh").is_some());
        assert!(paths.get("/api/v1/auth/logout").is_some());
        assert!(paths.get("/api/v1/me").is_some());
    }

    #[tokio::test]
    async fn auth_send_verify_me_flow() {
        let state = AppState::dev();
        let app = build_app_with_state(state);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"flow@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"flow@example.com","code":"123456"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        let access = json["access_token"].as_str().unwrap().to_string();
        let refresh = json["refresh_token"].as_str().unwrap().to_string();
        assert!(!access.is_empty());
        assert_eq!(json["expires_in"], 900);
        assert_eq!(json["user"]["email"], "flow@example.com");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {access}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let me = body_json(res).await;
        assert_eq!(me["email"], "flow@example.com");
        assert_eq!(me["display_name"], "flow");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/token/refresh")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"refresh_token":"{refresh}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let refreshed = body_json(res).await;
        let new_access = refreshed["access_token"].as_str().unwrap().to_string();
        let new_refresh = refreshed["refresh_token"].as_str().unwrap().to_string();
        assert!(!new_access.is_empty());

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/logout")
                    .header("authorization", format!("Bearer {new_access}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"refresh_token":"{new_refresh}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_otp_returns_auth_invalid_otp() {
        let app = build_app();
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"x@example.com","code":"000000"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(res).await;
        assert_eq!(json["code"], "AUTH_INVALID_OTP");
    }

    #[tokio::test]
    async fn me_rejects_missing_bearer() {
        let app = build_app();
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(res).await;
        assert_eq!(json["code"], "UNAUTHORIZED");
    }
}
