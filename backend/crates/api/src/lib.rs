//! HTTP routes and OpenAPI surface for AnyLive API.

mod auth_user;
pub mod cors;
mod error;
pub mod guards;
mod invite;
mod interactive;
mod analytics;
mod features;
mod oauth;
mod object_storage;
mod presence;
mod profile;
mod push;
mod push_delivery;
pub mod rate_limit;
mod recording;
mod rooms;
mod routes;
mod state;
mod tracing_init;

use std::sync::Arc;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;

pub use auth_user::AuthUser;
pub use cors::{cors_layer_for_env, cors_layer_from_env, is_permissive_cors};
pub use error::ApiError;
pub use guards::{
    check_centrifugo_for_production, check_jwt_secrets_for_production, check_otp_for_production,
    check_production_secrets, check_production_secrets_ext, constant_time_eq, is_local_env,
    is_production_env, mock_topup_allowed, DEFAULT_CENTRIFUGO_TOKEN_SECRET,
    DEFAULT_JWT_ACCESS_SECRET, DEFAULT_JWT_REFRESH_SECRET,
};
pub use rate_limit::IpRateLimiter;
pub use state::AppState;
pub use tracing_init::init_tracing;

/// Build the Axum router with in-memory auth (binary + integration tests).
pub fn build_app() -> Router {
    build_app_with_state(AppState::dev())
}

/// Build router with explicit shared state (permissive CORS for offline tests).
pub fn build_app_with_state(state: Arc<AppState>) -> Router {
    build_app_with_state_and_cors(state, CorsLayer::permissive())
}

/// Build router with explicit shared state and CORS layer (binary uses env CORS).
pub fn build_app_with_state_and_cors(state: Arc<AppState>, cors: CorsLayer) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/ready", get(routes::ready))
        .route("/metrics", get(routes::metrics))
        .route("/api/v1/meta", get(routes::meta))
        .route("/api/v1/auth/otp/send", post(routes::otp_send))
        .route("/api/v1/auth/otp/verify", post(routes::otp_verify))
        .route(
            "/api/v1/auth/password/login",
            post(routes::password_login),
        )
        .route(
            "/api/v1/auth/password/change",
            post(routes::password_change),
        )
        .route(
            "/api/v1/auth/oauth/exchange",
            post(routes::oauth_exchange),
        )
        .route("/api/v1/auth/token/refresh", post(routes::token_refresh))
        .route("/api/v1/auth/logout", post(routes::logout))
        .route(
            "/api/v1/me/sessions",
            get(routes::list_sessions).delete(routes::logout_all_sessions),
        )
        .route(
            "/api/v1/me/sessions/{jti}",
            delete(routes::revoke_session),
        )
        .route(
            "/api/v1/me/push-tokens",
            get(routes::list_push_tokens)
                .post(routes::register_push_token)
                .delete(routes::unregister_push_token),
        )
        .route(
            "/api/v1/me/push-tokens/test",
            post(routes::test_push),
        )
        .route(
            "/api/v1/me",
            get(routes::me)
                .patch(routes::patch_me)
                .delete(routes::delete_me),
        )
        .route("/api/v1/me/avatar/presign", post(routes::avatar_presign))
        .route("/api/v1/me/avatar/confirm", post(routes::avatar_confirm))
        .route("/api/v1/me/avatar/blob", put(routes::avatar_blob_put))
        .route("/api/v1/me/export", get(routes::export_me))
        .route("/api/v1/me/creator", get(routes::creator_stats))
        .route("/api/v1/legal/privacy", get(routes::legal_privacy))
        .route("/api/v1/legal/terms", get(routes::legal_terms))
        .route("/api/v1/rooms", post(routes::create_room).get(routes::list_rooms))
        .route("/api/v1/rooms/{id}", get(routes::get_room))
        .route("/api/v1/rooms/{id}/start", post(routes::start_room))
        .route("/api/v1/rooms/{id}/stop", post(routes::stop_room))
        .route(
            "/api/v1/rooms/{id}/stats",
            get(routes::room_stats),
        )
        .route(
            "/api/v1/rooms/{id}/presence",
            post(routes::room_presence_heartbeat),
        )
        .route(
            "/api/v1/rooms/{id}/likes",
            post(routes::room_like),
        )
        .route(
            "/api/v1/rooms/{id}/recording",
            get(routes::get_recording).put(routes::set_recording),
        )
        .route(
            "/api/v1/rooms/{id}/media/publish",
            post(routes::media_publish),
        )
        .route("/api/v1/rooms/{id}/media/play", get(routes::media_play))
        .route(
            "/api/v1/rooms/{id}/livekit/join",
            post(routes::livekit_join),
        )
        .route(
            "/api/v1/rooms/{id}/interactive/invite",
            post(routes::interactive_invite),
        )
        .route(
            "/api/v1/rooms/{id}/interactive/respond",
            post(routes::interactive_respond),
        )
        .route(
            "/api/v1/rooms/{id}/interactive/leave",
            post(routes::interactive_leave),
        )
        .route(
            "/api/v1/rooms/{id}/interactive",
            get(routes::list_interactive),
        )
        .route("/api/v1/rooms/{id}/pk", get(routes::get_pk))
        .route("/api/v1/rooms/{id}/pk/start", post(routes::start_pk))
        .route("/api/v1/rooms/{id}/pk/end", post(routes::end_pk))
        .route("/api/v1/events", post(routes::ingest_events))
        .route("/api/v1/wallet", get(routes::get_wallet))
        .route("/api/v1/wallet/ledger", get(routes::get_wallet_ledger))
        .route("/api/v1/wallet/topups", post(routes::topup_wallet))
        .route("/api/v1/pay/channels", get(routes::list_pay_channels))
        .route("/api/v1/pay/products", get(routes::list_pay_products))
        .route("/api/v1/pay/orders", post(routes::create_pay_order))
        .route("/api/v1/pay/orders/{id}", get(routes::get_pay_order))
        .route(
            "/api/v1/pay/orders/{id}/sandbox-complete",
            post(routes::sandbox_complete_pay_order),
        )
        .route("/api/v1/webhooks/pay/mock", post(routes::pay_webhook_mock))
        .route("/api/v1/webhooks/pay/jeepay", post(routes::pay_webhook_jeepay))
        .route("/api/v1/webhooks/pay/epay", post(routes::pay_webhook_epay))
        .route("/api/v1/webhooks/pay/tokenpay", post(routes::pay_webhook_tokenpay))
        .route("/api/v1/webhooks/pay/stripe", post(routes::pay_webhook_stripe))
        .route("/api/v1/webhooks/pay/iap", post(routes::pay_webhook_iap))
        .route("/api/v1/gifts", get(routes::list_gifts))
        .route("/api/v1/rooms/{id}/gifts", post(routes::send_gift))
        .route("/api/v1/realtime/token", post(routes::realtime_token))
        .route(
            "/api/v1/rooms/{id}/messages",
            post(routes::post_message).get(routes::list_messages),
        )
        .route("/api/v1/admin/grant", post(routes::grant_admin))
        .route("/api/v1/admin/ban", post(routes::ban_user))
        .route("/api/v1/admin/unban", post(routes::unban_user))
        .route("/api/v1/admin/mute", post(routes::mute_user))
        .route("/api/v1/admin/unmute", post(routes::unmute_user))
        .route(
            "/api/v1/admin/rooms/force-close",
            post(routes::force_close_room),
        )
        .route("/api/v1/admin/users/banned", get(routes::list_banned_users))
        .route("/api/v1/admin/users/muted", get(routes::list_muted_users))
        .route(
            "/api/v1/admin/users/{id}/moderation",
            get(routes::get_user_moderation),
        )
        .route("/api/v1/admin/audit", get(routes::list_audit))
        .route("/api/v1/admin/users", get(routes::admin_list_users).post(routes::admin_create_user))
        .route(
            "/api/v1/admin/users/{id}",
            get(routes::admin_get_user).patch(routes::admin_patch_user),
        )
        .route(
            "/api/v1/admin/users/{id}/reset-password",
            post(routes::admin_reset_password),
        )
        .route(
            "/api/v1/admin/users/{id}/revoke-sessions",
            post(routes::admin_revoke_sessions),
        )
        .route(
            "/api/v1/admin/wallet/reconcile",
            get(routes::wallet_reconcile),
        )
        .route(
            "/api/v1/admin/pay/expire-orders",
            post(routes::expire_pay_orders),
        )
        .route(
            "/api/v1/admin/analytics/summary",
            get(routes::analytics_summary),
        )
        .route(
            "/api/v1/users/{id}/follow",
            post(routes::follow_user).delete(routes::unfollow_user),
        )
        .route("/api/v1/me/following", get(routes::list_following))
        .route("/api/v1/feed/following", get(routes::feed_following))
        .route("/api/v1/feed/hot", get(routes::feed_hot))
        .route("/api/v1/search", get(routes::search))
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
        .layer(cors)
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
        routes::password_login,
        routes::password_change,
        routes::oauth_exchange,
        routes::token_refresh,
        routes::logout,
        routes::list_sessions,
        routes::logout_all_sessions,
        routes::revoke_session,
        routes::register_push_token,
        routes::list_push_tokens,
        routes::unregister_push_token,
        routes::test_push,
        routes::me,
        routes::patch_me,
        routes::avatar_presign,
        routes::avatar_confirm,
        routes::avatar_blob_put,
        routes::create_room,
        routes::list_rooms,
        routes::get_room,
        routes::start_room,
        routes::stop_room,
        routes::room_stats,
        routes::room_presence_heartbeat,
        routes::room_like,
        routes::get_recording,
        routes::set_recording,
        routes::media_publish,
        routes::media_play,
        routes::livekit_join,
        routes::interactive_invite,
        routes::interactive_respond,
        routes::interactive_leave,
        routes::list_interactive,
        routes::get_pk,
        routes::start_pk,
        routes::end_pk,
        routes::ingest_events,
        routes::creator_stats,
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
        routes::unban_user,
        routes::mute_user,
        routes::unmute_user,
        routes::force_close_room,
        routes::list_banned_users,
        routes::list_muted_users,
        routes::get_user_moderation,
        routes::list_audit,
        routes::admin_create_user,
        routes::admin_list_users,
        routes::admin_get_user,
        routes::admin_patch_user,
        routes::admin_reset_password,
        routes::admin_revoke_sessions,
        routes::wallet_reconcile,
        routes::expire_pay_orders,
        routes::analytics_summary,
        routes::follow_user,
        routes::unfollow_user,
        routes::list_following,
        routes::search,
    ),
    components(schemas(
        routes::HealthResponse,
        routes::MetaResponse,
        routes::MetaFeatures,
        routes::OtpSendBody,
        routes::OtpVerifyBody,
        routes::OauthExchangeBody,
        routes::RefreshBody,
        routes::LogoutBody,
        routes::TokenPairDto,
        routes::UserDto,
        routes::PatchMeBody,
        routes::AuthSessionResponse,
        routes::SessionDto,
        routes::SessionListResponse,
        routes::LogoutAllResponse,
        routes::PushRegisterBody,
        routes::PushDeviceDto,
        routes::PushDeviceListResponse,
        routes::PushTestBody,
        routes::PushTestResponse,
        crate::object_storage::AvatarPresignResponse,
        crate::object_storage::AvatarPresignHeader,
        crate::object_storage::AvatarPresignBody,
        crate::object_storage::AvatarConfirmBody,
        routes::AvatarBlobQuery,
        routes::CreateRoomBody,
        routes::RoomDto,
        routes::RoomListResponse,
        routes::RoomStatsDto,
        routes::PresenceHeartbeatResponse,
        routes::LikeRoomBody,
        routes::LikeRoomResponse,
        routes::RecordingStatusDto,
        routes::SetRecordingBody,
        routes::SearchQuerySchema,
        routes::SearchResponse,
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
        routes::UnbanUserBody,
        routes::MuteUserBody,
        routes::UnmuteUserBody,
        routes::ForceCloseBody,
        routes::GrantAdminBody,
        routes::AuditEventDto,
        routes::AuditListResponse,
        routes::ModerationEntryDto,
        routes::ModerationListResponse,
        routes::UserModerationStatusDto,
        routes::BalanceMismatchDto,
        routes::WalletReconcileResponse,
        routes::ExpirePayOrdersResponse,
        routes::AnalyticsNameCountDto,
        routes::AnalyticsRecentEventDto,
        routes::AnalyticsSummaryResponse,
        routes::FollowingListResponse,
        routes::LiveKitJoinBody,
        routes::LiveKitJoinDto,
        routes::InteractiveInviteBody,
        routes::InteractiveRespondBody,
        routes::InteractiveSessionDto,
        routes::InteractiveSessionListResponse,
        routes::StartPkBody,
        routes::PkSessionDto,
        routes::PkSessionResponse,
        routes::ClientEventDto,
        routes::ClientEventBatchBody,
        routes::ClientEventIngestResponse,
        routes::CreatorStatsResponse,
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
        (name = "social", description = "Follow graph"),
        (name = "analytics", description = "Client analytics events")
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
        assert_eq!(json["api_version"], "v1");
        assert_eq!(json["media_provider"], "srs");
        // build_app uses AppState::dev() → FeatureFlags::all_enabled() for tests.
        assert_eq!(json["features"]["pk"], true);
        assert_eq!(json["features"]["cohost"], true);
        assert_eq!(json["features"]["public_register"], true);
        assert_eq!(json["features"]["client_events"], true);
        assert_eq!(json["features"]["real_pay"], true);
    }

    #[tokio::test]
    async fn meta_reports_p1_safe_feature_defaults() {
        use std::sync::Arc;
        let base = AppState::dev();
        let flags = crate::features::FeatureFlags::default();
        assert!(!flags.pk && !flags.cohost);
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            base.centrifugo_publisher.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            base.invite.clone(),
            base.word_filter.clone(),
            base.livekit.clone(),
            base.interactive.clone(),
            base.analytics.clone(),
            flags,
        ));
        let app = build_app_with_state(state);
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
        assert_eq!(json["features"]["pk"], false);
        assert_eq!(json["features"]["cohost"], false);
        assert_eq!(json["features"]["public_register"], false);
        assert_eq!(json["features"]["client_events"], true);
    }

    #[tokio::test]
    async fn metrics_exposes_prometheus_text() {
        let app = build_app();
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("anylive_up 1"), "{text}");
        assert!(text.contains("anylive_http_requests_total"), "{text}");
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
        assert!(paths.get("/api/v1/auth/password/login").is_some());
        assert!(paths.get("/api/v1/auth/password/change").is_some());
        assert!(paths.get("/api/v1/auth/token/refresh").is_some());
        assert!(paths.get("/api/v1/auth/logout").is_some());
        assert!(paths.get("/api/v1/admin/users").is_some());
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

        // region patch (WBS E2.5)
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"region":"us"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let with_region = body_json(res).await;
        assert_eq!(with_region["region"], "US");

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

    #[tokio::test]
    async fn password_admin_create_login_and_ban_gate() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        // Bootstrap admin via OTP (dev flags allow public register).
        let admin_token = login(&app, "pw-admin@example.com").await;
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

        // Admin provisions host1 with password.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/users")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"display_name":"Host One","username":"host1","email":"host1@example.com","password":"secret-pass-1","must_change_password":false}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let created = body_json(res).await;
        assert_eq!(created["user"]["username"], "host1");
        assert_eq!(created["must_change_password"], false);
        let host_id = created["user"]["id"].as_str().unwrap().to_string();

        // Password login.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/password/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"identifier":"host1","password":"secret-pass-1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let session = body_json(res).await;
        assert!(!session["access_token"].as_str().unwrap().is_empty());
        assert_eq!(session["must_change_password"], false);
        assert_eq!(session["user"]["username"], "host1");

        // Wrong password → AUTH_INVALID_CREDENTIALS.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/password/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"identifier":"host1","password":"wrong-pass!!"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(res).await["code"], "AUTH_INVALID_CREDENTIALS");

        // Ban host → password login forbidden.
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
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/password/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"identifier":"host1","password":"secret-pass-1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // Unban restores login.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/unban")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"user_id":"{host_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/password/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"identifier":"host1@example.com","password":"secret-pass-1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
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
        let stream_key = pub_info["stream_key"].as_str().unwrap();
        assert!(
            stream_key.starts_with(&format!("{room_id}?")),
            "stream_key should be signed room?exp=&sig=, got {stream_key}"
        );
        assert!(
            stream_key.contains("exp=") && stream_key.contains("sig="),
            "stream_key must embed exp+sig query, got {stream_key}"
        );
        assert_ne!(stream_key, room_id, "stream_key must not be bare room uuid");
        assert!(pub_info["push_url"].as_str().unwrap().contains(&room_id));

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
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            recorder.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            base.invite.clone(),
            base.word_filter.clone(),
            base.livekit.clone(),
            base.interactive.clone(),
            base.analytics.clone(),
            base.features.clone(),
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
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            recorder.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            base.invite.clone(),
            base.word_filter.clone(),
            base.livekit.clone(),
            base.interactive.clone(),
            base.analytics.clone(),
            base.features.clone(),
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
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/users/banned")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let banned = body_json(res).await;
        assert!(banned["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["user_id"] == host_id));

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/admin/users/{host_id}/moderation"))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let status = body_json(res).await;
        assert_eq!(status["banned"], true);
        assert_eq!(status["ban_reason"], "spam");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/unban")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"user_id":"{host_id}","reason":"appeal"}}"#
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
        let items = audit["items"].as_array().unwrap();
        assert!(items.len() >= 3);
        assert!(items.iter().any(|e| e["action"] == "unban_user"));
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
    async fn admin_wallet_reconcile_balanced_after_topup() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let admin_token = login(&app, "reconcile-admin@example.com").await;
        let fan_token = login(&app, "reconcile-fan@example.com").await;
        bootstrap_admin(&app, &admin_token).await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":50,"reference":"reconcile-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/wallet/reconcile")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let report = body_json(res).await;
        assert_eq!(report["balanced"], true);
        assert_eq!(report["imbalance_count"], 0);
        assert!(report["checked_users"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn admin_analytics_summary_after_ingest() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let admin_token = login(&app, "analytics-admin@example.com").await;
        let fan_token = login(&app, "analytics-fan@example.com").await;
        bootstrap_admin(&app, &admin_token).await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("authorization", format!("Bearer {fan_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"events":[{"name":"room.view","props":{"room_id":"r1"}},{"name":"gift.tap","props":{"gift_id":"g1"}}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::ACCEPTED);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/analytics/summary")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let summary = body_json(res).await;
        assert!(summary["retained_events"].as_u64().unwrap() >= 2);
        assert!(summary["distinct_users"].as_u64().unwrap() >= 1);
        let names = summary["by_name"].as_array().unwrap();
        assert!(names.iter().any(|r| r["name"] == "room.view"));
    }

    #[tokio::test]
    async fn room_presence_likes_and_search_and_sessions() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let host = login(&app, "presence-host@example.com").await;
        let fan = login(&app, "presence-fan@example.com").await;

        // Create + start room
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Presence Lab"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let room = body_json(res).await;
        let room_id = room["id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/start"))
                    .header("authorization", format!("Bearer {host}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Heartbeats from host + fan
        for token in [&host, &fan] {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/rooms/{room_id}/presence"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_id}/stats"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let stats = body_json(res).await;
        assert_eq!(stats["online_count"].as_u64().unwrap(), 2);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/likes"))
                    .header("authorization", format!("Bearer {fan}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let like = body_json(res).await;
        assert_eq!(like["accepted"], true);
        assert_eq!(like["like_count"].as_u64().unwrap(), 1);

        // Search rooms by title substring
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/search?q=Presence&type=rooms")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let search = body_json(res).await;
        assert!(search["rooms"].as_array().unwrap().iter().any(|r| r["id"] == room_id));

        // Sessions list + logout-all
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me/sessions")
                    .header("authorization", format!("Bearer {fan}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let sessions = body_json(res).await;
        assert!(sessions["items"].as_array().unwrap().len() >= 1);
        let first_jti = sessions["items"][0]["jti"].as_str().unwrap().to_string();

        // Single-session revoke leaves remaining sessions intact when multiple exist;
        // after one login there is typically one session — re-login to get a second.
        let fan2 = login(&app, "presence-fan@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me/sessions")
                    .header("authorization", format!("Bearer {fan2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let sessions2 = body_json(res).await;
        assert!(sessions2["items"].as_array().unwrap().len() >= 1);
        let jti2 = sessions2["items"][0]["jti"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/me/sessions/{jti2}"))
                    .header("authorization", format!("Bearer {fan2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        // Revoking unknown jti → 404
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/me/sessions/{first_jti}"))
                    .header("authorization", format!("Bearer {fan2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // first_jti may belong to fan (same user after re-login same email shares user);
        // if still present and owned, could be 204; if already rotated away, 404. Either is fine
        // as long as status is one of those.
        assert!(
            res.status() == StatusCode::NO_CONTENT || res.status() == StatusCode::NOT_FOUND,
            "unexpected {}",
            res.status()
        );

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/me/sessions")
                    .header("authorization", format!("Bearer {fan2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let revoked = body_json(res).await;
        // may be 0 if all already revoked by single-jti path
        assert!(revoked["revoked"].as_u64().is_some());

        // Push token register / list / unregister (E8.9 scaffold)
        let fan_push = login(&app, "push-fan@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/me/push-tokens")
                    .header("authorization", format!("Bearer {fan_push}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"token":"dogfood-fcm-token","platform":"android"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let device = body_json(res).await;
        assert_eq!(device["token"], "dogfood-fcm-token");
        assert_eq!(device["platform"], "android");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me/push-tokens")
                    .header("authorization", format!("Bearer {fan_push}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let listed = body_json(res).await;
        assert_eq!(listed["items"].as_array().unwrap().len(), 1);

        // Test push via noop delivery before unregister.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/me/push-tokens/test")
                    .header("authorization", format!("Bearer {fan_push}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"hi","body":"dogfood"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let test_push = body_json(res).await;
        assert_eq!(test_push["delivery"], "noop");
        assert_eq!(test_push["attempted"], 1);
        assert_eq!(test_push["succeeded"], 1);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/me/push-tokens")
                    .header("authorization", format!("Bearer {fan_push}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"token":"dogfood-fcm-token","platform":"android"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn oauth_stub_exchange_issues_session() {
        let app = build_app();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/oauth/exchange")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"provider":"google","id_token":"stub:oauth-user@example.com"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert!(!body["access_token"].as_str().unwrap().is_empty());
        assert_eq!(body["user"]["email"], "oauth-user@example.com");

        // Real token (non-stub) is loud reject, not silent accept.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/oauth/exchange")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"provider":"apple","id_token":"eyJhbGciOiJSUzI1NiJ9.e30.sig"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn admin_grant_role_moderator() {
        let app = build_app();
        let admin_tok = login(&app, "role-admin@example.com").await;
        // bootstrap self
        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let me_body = body_json(me).await;
        let admin_id = me_body["id"].as_str().unwrap().to_string();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/grant")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"user_id":"{admin_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let mod_tok = login(&app, "role-mod@example.com").await;
        let mod_me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {mod_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let mod_body = body_json(mod_me).await;
        let mod_id = mod_body["id"].as_str().unwrap().to_string();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/grant")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"user_id":"{mod_id}","role":"moderator"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        // Moderator can mute (require_admin is full admin for ban still).
        // Mute still uses require_admin which is Admin rank — so moderator cannot ban.
        // Content path still require_admin for now (role matrix for mute is a follow-up).
        // Verify grant path accepted moderator role without error above.
        let _ = mod_id;
    }

    #[tokio::test]
    async fn avatar_presign_confirm_and_recording_toggle() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let host = login(&app, "avatar-host@example.com").await;

        // Presign avatar
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/me/avatar/presign")
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content_type":"image/jpeg"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let presign = body_json(res).await;
        let object_key = presign["object_key"].as_str().unwrap().to_string();
        let public_url = presign["public_url"].as_str().unwrap().to_string();
        assert!(object_key.starts_with("avatars/"));
        assert!(presign["upload_url"].as_str().unwrap().contains("token="));

        // Confirm without actually PUTting bytes (control plane).
        let body = serde_json::json!({
            "object_key": object_key,
            "public_url": public_url,
        });
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/me/avatar/confirm")
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let me = body_json(res).await;
        assert_eq!(me["avatar_url"].as_str().unwrap(), public_url);

        // Room recording toggle (owner only)
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Rec Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_id}/recording"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let rec = body_json(res).await;
        assert_eq!(rec["recording_enabled"], false);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/rooms/{room_id}/recording"))
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"enabled":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let rec = body_json(res).await;
        assert_eq!(rec["recording_enabled"], true);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_id}/stats"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let stats = body_json(res).await;
        assert_eq!(stats["recording_enabled"], true);
    }

    #[tokio::test]
    async fn invite_only_blocks_unknown_email() {
        use crate::invite::InviteGate;
        use std::sync::Arc;

        let base = AppState::dev();
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            base.centrifugo_publisher.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            InviteGate::restricted(&["allowed@example.com"], &["VIP"]),
            anylive_moderation::WordFilter::empty(),
            None,
            crate::interactive::InteractiveStore::new(),
            crate::analytics::AnalyticsStore::new(),
            crate::features::FeatureFlags::all_enabled(),
        ));
        let app = build_app_with_state(state);

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"blocked@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"blocked@example.com","code":"123456"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"blocked@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"blocked@example.com","code":"123456","invite_code":"VIP"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/send")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"allowed@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"allowed@example.com","code":"123456"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
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

        // idle bare UUID -> deny (code 1); bare UUIDs are never valid publish keys now
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

        // Issue signed publish stream key (required by webhook).
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
        let stream_key = body_json(res).await["stream_key"]
            .as_str()
            .unwrap()
            .to_string();

        // bare UUID still denied while live
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

        // live + signed key -> allow (code 0)
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/webhooks/srs/on_publish")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"stream":"{stream_key}"}}"#)))
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


    #[tokio::test]
    async fn interactive_invite_accept_and_pk_scores_gifts() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let host_a = login(&app, "pk-host-a@example.com").await;
        let host_b = login(&app, "pk-host-b@example.com").await;
        let guest = login(&app, "cohost-guest@example.com").await;

        // Resolve guest user id from /me
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {guest}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let guest_id = body_json(res).await["id"].as_str().unwrap().to_string();

        // Create + start two live rooms
        async fn create_start(app: &axum::Router, token: &str, title: &str) -> String {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/rooms")
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"title":"{title}"}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
            let id = body_json(res).await["id"].as_str().unwrap().to_string();
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/rooms/{id}/start"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            id
        }
        let room_a = create_start(&app, &host_a, "PK A").await;
        let room_b = create_start(&app, &host_b, "PK B").await;

        // Co-host invite
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_a}/interactive/invite"))
                    .header("authorization", format!("Bearer {host_a}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"invitee_id":"{guest_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(body_json(res).await["status"], "invited");

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_a}/interactive/respond"))
                    .header("authorization", format!("Bearer {guest}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"accept":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["status"], "active");

        // Start PK
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_a}/pk/start"))
                    .header("authorization", format!("Bearer {host_a}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"opponent_room_id":"{room_b}","duration_secs":120}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let pk = body_json(res).await;
        assert_eq!(pk["status"], "active");
        assert_eq!(pk["score_a"], 0);

        // Host A id for gift receiver
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {host_a}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let host_a_id = body_json(res).await["id"].as_str().unwrap().to_string();

        // Fan tops up and gifts into room A → PK score
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {guest}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":1000,"reference":"pk-topup-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success(), "topup {:?}", res.status());

        let gifts = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/gifts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let gifts_json = body_json(gifts).await;
        let gift_id = gifts_json["items"][0]["id"].as_str().unwrap();
        let gift_price = gifts_json["items"][0]["price"].as_i64().unwrap();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_a}/gifts"))
                    .header("authorization", format!("Bearer {guest}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"gift_id":"{gift_id}","receiver_id":"{host_a_id}","count":1,"client_request_id":"pk-gift-1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            res.status() == StatusCode::CREATED || res.status() == StatusCode::OK,
            "gift status {}",
            res.status()
        );

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/rooms/{room_a}/pk"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let pk = body_json(res).await;
        assert_eq!(pk["session"]["score_a"], gift_price);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_a}/pk/end"))
                    .header("authorization", format!("Bearer {host_a}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["status"], "ended");
    }

    #[tokio::test]
    async fn chat_word_filter_blocks_blocked_term() {
        use std::sync::Arc;
        let base = AppState::dev();
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            base.centrifugo_publisher.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            base.invite.clone(),
            anylive_moderation::WordFilter::from_words(["spamword"]),
            base.livekit.clone(),
            base.interactive.clone(),
            base.analytics.clone(),
            base.features.clone(),
        ));
        let app = build_app_with_state(state);
        let token = login(&app, "filter@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Filter Room"}"#))
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
                    .body(Body::from(r#"{"body":"buy SPAMword now"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(res).await["code"], "FORBIDDEN_POLICY");
    }

    #[tokio::test]
    async fn livekit_join_requires_config() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "lk@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"LK"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/livekit/join"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"role":"viewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        // MediaProviderError maps to 500
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body_json(res).await["code"], "MEDIA_PROVIDER_ERROR");
    }

    #[tokio::test]
    async fn livekit_join_mints_token_when_configured() {
        use std::sync::Arc;
        let base = AppState::dev();
        let lk = anylive_media::LiveKitProvider::new(
            "ws://localhost:7880",
            "devkey",
            "secret",
        );
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            base.centrifugo_publisher.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            base.invite.clone(),
            base.word_filter.clone(),
            Some(lk),
            base.interactive.clone(),
            base.analytics.clone(),
            base.features.clone(),
        ));
        let app = build_app_with_state(state);
        let token = login(&app, "lk-ok@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"LK OK"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_id = body_json(res).await["id"].as_str().unwrap().to_string();
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/livekit/join"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"role":"host"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["url"], "ws://localhost:7880");
        assert!(json["token"].as_str().unwrap().split('.').count() == 3);
        assert!(json["room_name"].as_str().unwrap().starts_with("room-"));
    }

    #[tokio::test]
    async fn events_ingest_accepts_batch() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let token = login(&app, "events@example.com").await;
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"events":[{"name":"room.view","client_event_id":"c1","props":{"room":"x"}},{"name":"room.view","client_event_id":"c1"}]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::ACCEPTED);
        let json = body_json(res).await;
        assert_eq!(json["accepted"], 1);
        assert_eq!(json["dropped"], 1);
    }

    #[test]
    fn openapi_contains_p3_paths() {
        let doc = openapi_json();
        let paths = doc.get("paths").unwrap();
        assert!(paths.get("/api/v1/rooms/{id}/livekit/join").is_some());
        assert!(paths.get("/api/v1/rooms/{id}/interactive/invite").is_some());
        assert!(paths.get("/api/v1/rooms/{id}/pk/start").is_some());
        assert!(paths.get("/api/v1/events").is_some());
    }


    #[tokio::test]
    async fn creator_stats_reflects_followers_and_gifts() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state);
        let host = login(&app, "creator-host@example.com").await;
        let fan = login(&app, "creator-fan@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {host}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let host_id = body_json(res).await["id"].as_str().unwrap().to_string();

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/users/{host_id}/follow"))
                    .header("authorization", format!("Bearer {fan}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Creator Show"}"#))
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
                    .header("authorization", format!("Bearer {host}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {fan}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":500,"reference":"creator-topup"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let gifts = app
            .clone()
            .oneshot(Request::builder().uri("/api/v1/gifts").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let gifts_json = body_json(gifts).await;
        let gift_id = gifts_json["items"][0]["id"].as_str().unwrap();
        let price = gifts_json["items"][0]["price"].as_i64().unwrap();
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/gifts"))
                    .header("authorization", format!("Bearer {fan}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"gift_id":"{gift_id}","receiver_id":"{host_id}","count":1,"client_request_id":"creator-g1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me/creator")
                    .header("authorization", format!("Bearer {host}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let stats = body_json(res).await;
        assert_eq!(stats["follower_count"], 1);
        assert_eq!(stats["live_rooms"], 1);
        assert_eq!(stats["total_rooms"], 1);
        assert_eq!(stats["gift_coins_received"], price);
        assert_eq!(stats["gift_credit_entries"], 1);
    }

    #[tokio::test]
    async fn feed_hot_ranks_by_followers() {
        let state = AppState::dev();
        let app = build_app_with_state(state);
        let popular = login(&app, "hot-popular@example.com").await;
        let quiet = login(&app, "hot-quiet@example.com").await;
        let fan1 = login(&app, "hot-fan1@example.com").await;
        let fan2 = login(&app, "hot-fan2@example.com").await;

        async fn me_id(app: &axum::Router, token: &str) -> String {
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
            body_json(res).await["id"].as_str().unwrap().to_string()
        }
        let popular_id = me_id(&app, &popular).await;
        let quiet_id = me_id(&app, &quiet).await;

        for fan in [&fan1, &fan2] {
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/users/{popular_id}/follow"))
                        .header("authorization", format!("Bearer {fan}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/users/{quiet_id}/follow"))
                    .header("authorization", format!("Bearer {fan1}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        async fn start_room(app: &axum::Router, token: &str, title: &str) -> String {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/rooms")
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"title":"{title}"}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
            let id = body_json(res).await["id"].as_str().unwrap().to_string();
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/rooms/{id}/start"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            id
        }
        let pop_room = start_room(&app, &popular, "Popular Live").await;
        let quiet_room = start_room(&app, &quiet, "Quiet Live").await;

        let res = app
            .oneshot(Request::builder().uri("/api/v1/feed/hot").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let items = body_json(res).await["items"].as_array().unwrap().clone();
        let ids: Vec<&str> = items
            .iter()
            .filter_map(|i| i["id"].as_str())
            .collect();
        let pop_pos = ids.iter().position(|id| *id == pop_room).expect("popular room");
        let quiet_pos = ids.iter().position(|id| *id == quiet_room).expect("quiet room");
        assert!(pop_pos < quiet_pos, "popular should rank above quiet: {ids:?}");
    }


    #[tokio::test]
    async fn feature_flag_blocks_pk_start() {
        use std::sync::Arc;
        let base = AppState::dev();
        let mut flags = base.features.clone();
        flags.pk = false;
        let state = Arc::new(AppState::new(
            base.auth.clone(),
            base.rooms.clone(),
            base.media.clone(),
            base.wallet.clone(),
            base.chat.clone(),
            base.chat_rate_limiter.clone(),
            base.otp_ip_limiter.clone(),
            base.centrifugo.clone(),
            base.centrifugo_publisher.clone(),
            base.moderation.clone(),
            base.social.clone(),
            base.reports.clone(),
            base.deleted_users.clone(),
            base.profile_extras.clone(),
            None,
            base.allow_mock_topup,
            base.pay.clone(),
            base.pay_registry.clone(),
            base.pay_public_base.clone(),
            base.pay_mock_secret.clone(),
            base.pay_sandbox_limiter.clone(),
            base.invite.clone(),
            base.word_filter.clone(),
            base.livekit.clone(),
            base.interactive.clone(),
            base.analytics.clone(),
            flags,
        ));
        let app = build_app_with_state(state);
        let host = login(&app, "flag-pk@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Flag PK"}"#))
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
                    .header("authorization", format!("Bearer {host}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let other = login(&app, "flag-pk-b@example.com").await;
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {other}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Flag PK B"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let room_b = body_json(res).await["id"].as_str().unwrap().to_string();
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_b}/start"))
                    .header("authorization", format!("Bearer {other}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/rooms/{room_id}/pk/start"))
                    .header("authorization", format!("Bearer {host}"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"opponent_room_id":"{room_b}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(res).await["code"], "FORBIDDEN_POLICY");
    }

}
