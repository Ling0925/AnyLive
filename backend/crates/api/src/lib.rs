//! HTTP routes and OpenAPI surface for AnyLive API.

mod auth_user;
mod error;
mod rooms;
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
        .route("/api/v1/rooms", post(routes::create_room).get(routes::list_rooms))
        .route("/api/v1/rooms/{id}", get(routes::get_room))
        .route("/api/v1/rooms/{id}/start", post(routes::start_room))
        .route("/api/v1/rooms/{id}/stop", post(routes::stop_room))
        .route(
            "/api/v1/rooms/{id}/media/publish",
            post(routes::media_publish),
        )
        .route("/api/v1/rooms/{id}/media/play", get(routes::media_play))
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
        routes::create_room,
        routes::list_rooms,
        routes::get_room,
        routes::start_room,
        routes::stop_room,
        routes::media_publish,
        routes::media_play,
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
        routes::CreateRoomBody,
        routes::RoomDto,
        routes::RoomListResponse,
        routes::PublishInfoDto,
        routes::PlayUrlsDto,
    )),
    tags(
        (name = "system", description = "Health and metadata"),
        (name = "auth", description = "Authentication"),
        (name = "users", description = "User profile"),
        (name = "rooms", description = "Rooms and media control plane")
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

    async fn login(app: &axum::Router, email: &str) -> String {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","code":"123456"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        json["access_token"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn rooms_create_start_publish_play_stop() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "host@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Friday Show"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        assert_eq!(room["status"], "idle");

        // play while idle -> ROOM_NOT_LIVE
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_id}/media/play"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let err = body_json(res).await;
        assert_eq!(err["code"], "ROOM_NOT_LIVE");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["status"], "live");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/media/publish"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let pub_info = body_json(res).await;
        assert!(pub_info["push_url"].as_str().unwrap().contains(&room_id));
        assert_eq!(pub_info["stream_key"], room_id);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_id}/media/play"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let play = body_json(res).await;
        assert!(play["hls"].as_str().unwrap().ends_with(".m3u8"));

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/stop"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["status"], "idle");
    }

    #[test]
    fn openapi_contains_room_paths() {
        let doc = openapi_json();
        let paths = doc.get("paths").unwrap();
        assert!(paths.get("/api/v1/rooms").is_some());
        assert!(paths.get("/api/v1/rooms/{id}/media/play").is_some());
    }
}
