//! HTTP routes and OpenAPI surface for AnyLive API.

mod auth_user;
mod error;
pub mod guards;
mod profile;
mod rooms;
mod routes;
mod state;

use std::sync::Arc;

use axum::{
    routing::{get, patch, post},
    Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub use auth_user::AuthUser;
pub use error::ApiError;
pub use guards::{
    check_centrifugo_for_production, check_jwt_secrets_for_production, check_otp_for_production,
    check_production_secrets, is_production_env, DEFAULT_CENTRIFUGO_TOKEN_SECRET,
    DEFAULT_JWT_ACCESS_SECRET, DEFAULT_JWT_REFRESH_SECRET,
};
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
        .route(
            "/api/v1/me",
            get(routes::me)
                .patch(routes::patch_me)
                .delete(routes::delete_me),
        )
        .route("/api/v1/me/export", get(routes::export_me))
        .route("/api/v1/legal/privacy", get(routes::legal_privacy))
        .route("/api/v1/legal/terms", get(routes::legal_terms))
        .route("/api/v1/rooms", post(routes::create_room).get(routes::list_rooms))
        .route("/api/v1/rooms/{id}", get(routes::get_room))
        .route("/api/v1/rooms/{id}/start", post(routes::start_room))
        .route("/api/v1/rooms/{id}/stop", post(routes::stop_room))
        .route(
            "/api/v1/rooms/{id}/media/publish",
            post(routes::media_publish),
        )
        .route("/api/v1/rooms/{id}/media/play", get(routes::media_play))
        .route("/api/v1/wallet", get(routes::get_wallet))
        .route("/api/v1/wallet/ledger", get(routes::get_wallet_ledger))
        .route("/api/v1/wallet/topups", post(routes::topup_wallet))
        .route("/api/v1/gifts", get(routes::list_gifts))
        .route("/api/v1/rooms/{id}/gifts", post(routes::send_gift))
        .route("/api/v1/realtime/token", post(routes::realtime_token))
        .route(
            "/api/v1/rooms/{id}/messages",
            post(routes::post_message).get(routes::list_messages),
        )
        .route("/api/v1/admin/grant", post(routes::grant_admin))
        .route("/api/v1/admin/ban", post(routes::ban_user))
        .route("/api/v1/admin/mute", post(routes::mute_user))
        .route("/api/v1/admin/unmute", post(routes::unmute_user))
        .route(
            "/api/v1/admin/rooms/force-close",
            post(routes::force_close_room),
        )
        .route("/api/v1/admin/audit", get(routes::list_audit))
        .route(
            "/api/v1/users/{id}/follow",
            post(routes::follow_user).delete(routes::unfollow_user),
        )
        .route("/api/v1/me/following", get(routes::list_following))
        .route("/api/v1/feed/following", get(routes::feed_following))
        .route("/api/v1/feed/hot", get(routes::feed_hot))
        .route("/api/v1/reports", post(routes::create_report))
        .route("/api/v1/admin/gifts", get(routes::admin_list_gifts).post(routes::admin_upsert_gift))
        .route("/api/v1/admin/reports", get(routes::admin_list_reports))
        .route(
            "/api/v1/admin/reports/{id}",
            patch(routes::admin_resolve_report),
        )
        .route("/api/v1/webhooks/srs/on_publish", post(routes::srs_on_publish))
        .route("/api/v1/webhooks/srs/on_unpublish", post(routes::srs_on_unpublish))
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
        routes::patch_me,
        routes::create_room,
        routes::list_rooms,
        routes::get_room,
        routes::start_room,
        routes::stop_room,
        routes::media_publish,
        routes::media_play,
        routes::get_wallet,
        routes::get_wallet_ledger,
        routes::topup_wallet,
        routes::list_gifts,
        routes::send_gift,
        routes::realtime_token,
        routes::post_message,
        routes::list_messages,
        routes::grant_admin,
        routes::ban_user,
        routes::mute_user,
        routes::unmute_user,
        routes::force_close_room,
        routes::list_audit,
        routes::follow_user,
        routes::unfollow_user,
        routes::list_following,
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
        routes::PatchMeBody,
        routes::AuthSessionResponse,
        routes::CreateRoomBody,
        routes::RoomDto,
        routes::RoomListResponse,
        routes::PublishInfoDto,
        routes::PlayUrlsDto,
        routes::WalletDto,
        routes::LedgerEntryDto,
        routes::LedgerListResponse,
        routes::TopupBody,
        routes::GiftDto,
        routes::GiftListResponse,
        routes::SendGiftBody,
        routes::GiftOrderDto,
        routes::RealtimeTokenBody,
        routes::RealtimeTokenResponse,
        routes::PostMessageBody,
        routes::ChatMessageDto,
        routes::ChatListResponse,
        routes::BanUserBody,
        routes::MuteUserBody,
        routes::UnmuteUserBody,
        routes::ForceCloseBody,
        routes::GrantAdminBody,
        routes::AuditEventDto,
        routes::AuditListResponse,
        routes::FollowingListResponse,
    )),
    tags(
        (name = "system", description = "Health and metadata"),
        (name = "auth", description = "Authentication"),
        (name = "users", description = "User profile"),
        (name = "rooms", description = "Rooms and media control plane"),
        (name = "wallet", description = "Virtual currency wallet"),
        (name = "gifts", description = "Gift catalog and send"),
        (name = "realtime", description = "Chat and Centrifugo tokens"),
        (name = "admin", description = "Moderation"),
        (name = "social", description = "Follow graph")
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
    async fn patch_me_updates_display_name_and_declarations() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "patchme@example.com").await;

        // defaults false
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let me = body_json(res).await;
        assert_eq!(me["age_confirmed"], false);
        assert_eq!(me["privacy_accepted"], false);
        assert_eq!(me["display_name"], "patchme");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"display_name":"Patched","age_confirmed":true,"privacy_accepted":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let patched = body_json(res).await;
        assert_eq!(patched["display_name"], "Patched");
        assert_eq!(patched["age_confirmed"], true);
        assert_eq!(patched["privacy_accepted"], true);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let me2 = body_json(res).await;
        assert_eq!(me2["display_name"], "Patched");
        assert_eq!(me2["age_confirmed"], true);
        assert_eq!(me2["privacy_accepted"], true);

        // empty body rejected
        let res = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
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

    #[tokio::test]
    async fn feed_hot_lists_live_rooms() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "hot@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Hot Show"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();
        let _ = app
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
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/feed/hot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let feed = body_json(res).await;
        assert!(feed["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == room_id));
    }

    #[tokio::test]
    async fn gift_send_idempotent_http() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let host_token = login(&app, "host2@example.com").await;
        let fan_token = login(&app, "fan@example.com").await;

        // create room as host
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Gift Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        let owner_id = room["owner_id"].as_str().unwrap().to_string();

        // Gifts require a live room.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // topup fan
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":50}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["balance"], 50);

        // list gifts
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/gifts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let gifts = body_json(res).await;
        let gift_id = gifts["items"][0]["id"].as_str().unwrap().to_string();

        let body = format!(
            r#"{{"gift_id":"{gift_id}","receiver_id":"{owner_id}","count":2,"client_request_id":"idem-1"}}"#
        );
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let order = body_json(res).await;
        assert_eq!(order["replayed"], false);
        let order_id = order["id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let order2 = body_json(res).await;
        assert_eq!(order2["replayed"], true);
        assert_eq!(order2["id"], order_id);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // 50 - 2*price(rose=1) = 48
        assert_eq!(body_json(res).await["balance"], 48);
    }

    #[tokio::test]
    async fn wallet_ledger_lists_topup_and_gift_debit() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let host_token = login(&app, "ledger-host@example.com").await;
        let fan_token = login(&app, "ledger-fan@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Ledger Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        let owner_id = room["owner_id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":100}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["balance"], 100);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/gifts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let gifts = body_json(res).await;
        let gift = gifts["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["name"] == "Rose")
            .expect("rose gift");
        let gift_id = gift["id"].as_str().unwrap().to_string();
        assert_eq!(gift["active"], true);

        let body = format!(
            r#"{{"gift_id":"{gift_id}","receiver_id":"{owner_id}","count":3,"client_request_id":"ledger-1"}}"#
        );
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // Fan ledger: topup credit + gift debit
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet/ledger")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let fan_ledger = body_json(res).await;
        let fan_items = fan_ledger["items"].as_array().unwrap();
        assert!(fan_items.iter().any(|e| {
            e["entry_type"] == "topup" && e["amount"] == 100 && e["balance_after"] == 100
        }));
        assert!(fan_items.iter().any(|e| {
            e["entry_type"] == "gift_debit" && e["amount"] == -3 && e["balance_after"] == 97
        }));

        // Host ledger: gift credit
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallet/ledger")
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let host_ledger = body_json(res).await;
        let host_items = host_ledger["items"].as_array().unwrap();
        assert!(host_items.iter().any(|e| {
            e["entry_type"] == "gift_credit" && e["amount"] == 3 && e["balance_after"] == 3
        }));
    }

    #[tokio::test]
    async fn chat_post_and_history() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "chat@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Chat Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/messages"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"hello live"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_id}/messages"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let list = body_json(res).await;
        assert_eq!(list["items"][0]["body"], "hello live");

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/realtime/token")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"room_id":"{room_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let tok = body_json(res).await;
        assert!(!tok["token"].as_str().unwrap().is_empty());
        assert!(tok["channels"][0]
            .as_str()
            .unwrap()
            .starts_with("room:"));
    }

    #[tokio::test]
    async fn chat_post_rate_limited_on_sixth_burst() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "chat-rate@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Rate Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();

        // Default limit: 5 messages per 10s. First five succeed.
        for i in 0..5 {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/rooms/{room_id}/messages"))
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"body":"msg {i}"}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                res.status(),
                StatusCode::CREATED,
                "message {i} should be allowed"
            );
        }

        // Sixth in the same burst is rate limited.
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/messages"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"too many"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        let err = body_json(res).await;
        assert_eq!(err["code"], "RATE_LIMITED");
        assert_eq!(err["retryable"], true);
    }

    #[tokio::test]
    async fn chat_post_publishes_centrifugo_envelope() {
        use anylive_realtime::RecordingCentrifugoPublisher;
        use std::sync::Arc;

        let recorder = Arc::new(RecordingCentrifugoPublisher::new());
        let base = AppState::dev();
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.centrifugo.clone(),
            recorder.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
        ));
        let app = build_app_with_state(state);
        let token = login(&app, "chat-pub@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Pub Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/messages"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"fanout me"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let snap = recorder.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, format!("room:{room_id}"));
        assert_eq!(snap[0].1["type"], "chat.message");
        assert_eq!(snap[0].1["payload"]["body"], "fanout me");
        assert_eq!(snap[0].1["payload"]["room_id"], room_id);
    }

    #[tokio::test]
    async fn gift_send_publishes_centrifugo_envelope_once() {
        use anylive_realtime::RecordingCentrifugoPublisher;
        use std::sync::Arc;

        let recorder = Arc::new(RecordingCentrifugoPublisher::new());
        let base = AppState::dev_ready().await;
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.centrifugo.clone(),
            recorder.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
        ));
        let app = build_app_with_state(state);
        let host_token = login(&app, "gift-pub-host@example.com").await;
        let fan_token = login(&app, "gift-pub-fan@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Gift Pub Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        let owner_id = room["owner_id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":50}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/gifts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let gifts = body_json(res).await;
        let gift_id = gifts["items"][0]["id"].as_str().unwrap().to_string();

        let body = format!(
            r#"{{"gift_id":"{gift_id}","receiver_id":"{owner_id}","count":2,"client_request_id":"gift-pub-1"}}"#
        );
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let order = body_json(res).await;
        let order_id = order["id"].as_str().unwrap().to_string();
        let total_coins = order["total_coins"].as_i64().unwrap();

        // Idempotent replay must not re-publish.
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["replayed"], true);

        let snap = recorder.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, format!("room:{room_id}"));
        assert_eq!(snap[0].1["type"], "gift.sent");
        assert_eq!(snap[0].1["payload"]["order_id"], order_id);
        assert_eq!(snap[0].1["payload"]["room_id"], room_id);
        assert_eq!(snap[0].1["payload"]["receiver_id"], owner_id);
        assert_eq!(snap[0].1["payload"]["gift_id"], gift_id);
        assert_eq!(snap[0].1["payload"]["count"], 2);
        assert_eq!(snap[0].1["payload"]["total_coins"], total_coins);
    }

    #[tokio::test]
    async fn admin_ban_and_force_close() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let admin_token = login(&app, "admin@example.com").await;
        let host_token = login(&app, "stream@example.com").await;

        // bootstrap admin
        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let admin_id = body_json(me).await["id"].as_str().unwrap().to_string();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/grant")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"user_id":"{admin_id}"}}"#)))
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
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"To Close"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        let host_id = room["owner_id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/rooms/force-close")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"room_id":"{room_id}","reason":"violation"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["status"], "closed");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/ban")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"user_id":"{host_id}","reason":"spam"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/audit")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let audit = body_json(res).await;
        assert!(audit["items"].as_array().unwrap().len() >= 2);
    }

    async fn bootstrap_admin(app: &axum::Router, token: &str) {
        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let admin_id = body_json(me).await["id"].as_str().unwrap().to_string();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/grant")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"user_id":"{admin_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_mute_blocks_chat_and_gifts() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let admin_token = login(&app, "mute-admin@example.com").await;
        let host_token = login(&app, "mute-host@example.com").await;
        let fan_token = login(&app, "mute-fan@example.com").await;
        bootstrap_admin(&app, &admin_token).await;

        // host creates room
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Mute Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        let owner_id = room["owner_id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // resolve fan id
        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let fan_id = body_json(me).await["id"].as_str().unwrap().to_string();

        // topup + gift catalog
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":50}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/gifts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let gifts = body_json(res).await;
        let gift_id = gifts["items"][0]["id"].as_str().unwrap().to_string();

        // mute fan
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/mute")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"user_id":"{fan_id}","reason":"spam"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        // chat blocked
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/messages"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"should fail"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let err = body_json(res).await;
        assert_eq!(err["code"], "FORBIDDEN");

        // gift blocked
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"gift_id":"{gift_id}","receiver_id":"{owner_id}","count":1,"client_request_id":"mute-1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // unmute restores chat
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/unmute")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"user_id":"{fan_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/messages"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"ok after unmute"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn admin_upsert_gift_and_list() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let admin_token = login(&app, "gift-admin@example.com").await;
        bootstrap_admin(&app, &admin_token).await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/gifts")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Rocket","price":99,"active":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let gift = body_json(res).await;
        assert_eq!(gift["name"], "Rocket");
        assert_eq!(gift["price"], 99);
        let gift_id = gift["id"].as_str().unwrap().to_string();

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/gifts")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let list = body_json(res).await;
        assert!(list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["id"] == gift_id && g["name"] == "Rocket"));
    }

    #[tokio::test]
    async fn create_report_and_admin_list() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let admin_token = login(&app, "report-admin@example.com").await;
        let reporter_token = login(&app, "reporter@example.com").await;
        bootstrap_admin(&app, &admin_token).await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/reports")
                    .header("authorization", format!("Bearer {reporter_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"target_type":"user","target_id":"some-user","reason":"harassment"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let report = body_json(res).await;
        let report_id = report["id"].as_str().unwrap().to_string();
        assert_eq!(report["target_type"], "user");
        assert_eq!(report["reason"], "harassment");
        assert_eq!(report["status"], "open");

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/reports")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let list = body_json(res).await;
        assert!(list["items"].as_array().unwrap().iter().any(|r| {
            r["id"] == report_id && r["reason"] == "harassment" && r["status"] == "open"
        }));
    }

    #[tokio::test]
    async fn admin_resolve_report() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let admin_token = login(&app, "resolve-admin@example.com").await;
        let reporter_token = login(&app, "resolve-reporter@example.com").await;
        bootstrap_admin(&app, &admin_token).await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/reports")
                    .header("authorization", format!("Bearer {reporter_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"target_type":"room","target_id":"room-1","reason":"scam"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let report_id = body_json(res).await["id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/admin/reports/{report_id}"))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"status":"resolved","note":"banned host"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let resolved = body_json(res).await;
        assert_eq!(resolved["id"], report_id);
        assert_eq!(resolved["status"], "resolved");
        assert_eq!(resolved["note"], "banned host");

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/reports")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let list = body_json(res).await;
        assert!(list["items"].as_array().unwrap().iter().any(|r| {
            r["id"] == report_id && r["status"] == "resolved" && r["note"] == "banned host"
        }));
    }

    #[tokio::test]
    async fn srs_on_publish_allow_live_deny_idle() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "srs-host@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"SRS Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();

        // idle -> deny (code 1)
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/srs/on_publish")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"stream":"{room_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["code"], 1);

        // start live
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

        // live -> allow (code 0)
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/srs/on_publish")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"stream":"{room_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["code"], 0);
    }

    #[tokio::test]
    async fn feed_following_after_follow_live_host() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let host_token = login(&app, "follow-host@example.com").await;
        let fan_token = login(&app, "follow-fan@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Follow Show"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();
        let host_id = room["owner_id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // empty before follow
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/feed/following")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(body_json(res).await["items"].as_array().unwrap().is_empty());

        // follow host
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/users/{host_id}/follow"))
                    .header("authorization", format!("Bearer {fan_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/feed/following")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let feed = body_json(res).await;
        assert!(feed["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == room_id && r["status"] == "live"));
    }
}
