//! Shared API state.

use std::sync::Arc;

use anylive_auth::{
    AuthService, InMemoryOtpStore, InMemoryRefreshStore, JwtConfig, JwtService, OtpConfig,
    OtpService,
};
use anylive_db::{
    postgres_enabled, AnyChat, AnyModeration, AnyProfileExtras, AnyReports, AnySocial,
    AnyUserStore, AnyWallet, PgPool,
};
use anylive_media::SrsMediaProvider;
use anylive_realtime::{
    publisher_from_env, CentrifugoConfig, CentrifugoPublisher, ChatRateLimiter,
    NoopCentrifugoPublisher,
};

use crate::guards::{check_production_secrets, is_production_env};
use crate::rooms::AnyRoomStore;
use crate::routes::compliance::DeletedUsers;

/// Auth service with pluggable user store (memory default, Postgres when enabled).
pub type AppAuthService = AuthService<AnyUserStore, InMemoryOtpStore, InMemoryRefreshStore>;

#[derive(Clone)]
pub struct AppState {
    pub auth: AppAuthService,
    pub rooms: AnyRoomStore,
    pub media: SrsMediaProvider,
    pub wallet: AnyWallet,
    pub chat: AnyChat,
    /// Per-user chat post rate limiter (in-memory sliding window).
    pub chat_rate_limiter: ChatRateLimiter,
    pub centrifugo: CentrifugoConfig,
    /// Centrifugo HTTP API publisher (noop when env not set).
    pub centrifugo_publisher: Arc<dyn CentrifugoPublisher>,
    pub moderation: AnyModeration,
    pub social: AnySocial,
    pub reports: AnyReports,
    /// Soft-deleted accounts (P1 compliance stub).
    pub deleted_users: DeletedUsers,
    /// Age/privacy declarations (memory dual store; Postgres when USE_POSTGRES=1).
    pub profile_extras: AnyProfileExtras,
    /// Present when `USE_POSTGRES=1` + `DATABASE_URL` were used at startup.
    pub db: Option<PgPool>,
}

impl AppState {
    pub fn new(
        auth: AppAuthService,
        rooms: AnyRoomStore,
        media: SrsMediaProvider,
        wallet: AnyWallet,
        chat: AnyChat,
        chat_rate_limiter: ChatRateLimiter,
        centrifugo: CentrifugoConfig,
        centrifugo_publisher: Arc<dyn CentrifugoPublisher>,
        moderation: AnyModeration,
        social: AnySocial,
        reports: AnyReports,
        deleted_users: DeletedUsers,
        profile_extras: AnyProfileExtras,
        db: Option<PgPool>,
    ) -> Self {
        Self {
            auth,
            rooms,
            media,
            wallet,
            chat,
            chat_rate_limiter,
            centrifugo,
            centrifugo_publisher,
            moderation,
            social,
            reports,
            deleted_users,
            profile_extras,
            db,
        }
    }

    /// Local/dev defaults (fixed OTP `123456`, insecure JWT defaults allowed).
    /// Unchanged for integration tests and offline development — always memory stores.
    pub fn dev() -> Arc<Self> {
        let jwt = JwtService::new(JwtConfig::from_env());
        let otp = OtpService::new(InMemoryOtpStore::default(), OtpConfig::dev());
        let auth = AuthService::new(
            AnyUserStore::memory(),
            otp,
            InMemoryRefreshStore::default(),
            jwt,
        );
        Arc::new(Self::new(
            auth,
            AnyRoomStore::memory(),
            SrsMediaProvider::from_env(),
            AnyWallet::memory(),
            AnyChat::memory(),
            ChatRateLimiter::default(),
            CentrifugoConfig::default(),
            // Tests/offline: never require a live Centrifugo.
            Arc::new(NoopCentrifugoPublisher::new()),
            AnyModeration::memory(),
            AnySocial::memory(),
            AnyReports::memory(),
            DeletedUsers::new(),
            AnyProfileExtras::memory(),
            None,
        ))
    }

    /// Async init for tests/dev that need gift catalog.
    pub async fn dev_ready() -> Arc<Self> {
        let state = Self::dev();
        state.wallet.seed_default_gifts().await;
        state
    }

    /// Build state from environment with production startup guards.
    ///
    /// Reads:
    /// - `APP_ENV` — `production` / `prod` enables fail-closed secret checks
    /// - `JWT_ACCESS_SECRET` / `JWT_REFRESH_SECRET` via [`JwtConfig::from_env`]
    /// - `CENTRIFUGO_TOKEN_SECRET` via [`CentrifugoConfig::default`]
    /// - OTP: fixed/dev OTP only when **not** production
    /// - `USE_POSTGRES=1` + `DATABASE_URL` — optional Postgres dual stores + migrations
    ///
    /// Realtime routes are always mounted, so Centrifugo secret is always guarded
    /// in production.
    pub async fn from_env() -> Result<Arc<Self>, String> {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into());
        let jwt_cfg = JwtConfig::from_env();
        let centrifugo = CentrifugoConfig::default();
        // Secure default OTP outside production; dev fixed OTP only for non-prod.
        let otp_cfg = if is_production_env(&app_env) {
            OtpConfig::default()
        } else {
            OtpConfig::dev()
        };

        // Realtime token endpoint is always present in this binary.
        const REALTIME_USED: bool = true;
        check_production_secrets(
            &app_env,
            &jwt_cfg.access_secret,
            &jwt_cfg.refresh_secret,
            otp_cfg.dev_fixed_otp,
            Some(&centrifugo.token_secret),
            REALTIME_USED,
        )?;

        let jwt = JwtService::new(jwt_cfg);
        let otp = OtpService::new(InMemoryOtpStore::default(), otp_cfg);

        let (users, rooms, wallet, moderation, social, reports, chat, profile_extras, db) =
            if postgres_enabled() {
                let pool = anylive_db::connect_and_migrate_from_env()
                    .await
                    .map_err(|e| format!("postgres connect/migrate failed: {e}"))?;
                tracing::info!(
                    "postgres enabled: migrations applied; using Postgres dual stores for \
                     users/rooms/wallet/social/moderation/reports/chat/profile_extras"
                );
                (
                    AnyUserStore::postgres(pool.clone()),
                    AnyRoomStore::postgres(pool.clone()),
                    AnyWallet::postgres(pool.clone()),
                    AnyModeration::postgres(pool.clone()),
                    AnySocial::postgres(pool.clone()),
                    AnyReports::postgres(pool.clone()),
                    AnyChat::postgres(pool.clone()),
                    AnyProfileExtras::postgres(pool.clone()),
                    Some(pool),
                )
            } else {
                if is_production_env(&app_env) {
                    // Fail closed: production must not silently run on volatile memory stores.
                    return Err(
                        "production requires USE_POSTGRES=1 and DATABASE_URL (in-memory store forbidden)"
                            .into(),
                    );
                }
                tracing::info!(
                    "postgres disabled (set USE_POSTGRES=1 and DATABASE_URL to enable)"
                );
                (
                    AnyUserStore::memory(),
                    AnyRoomStore::memory(),
                    AnyWallet::memory(),
                    AnyModeration::memory(),
                    AnySocial::memory(),
                    AnyReports::memory(),
                    AnyChat::memory(),
                    AnyProfileExtras::memory(),
                    None,
                )
            };

        let auth = AuthService::new(users, otp, InMemoryRefreshStore::default(), jwt);

        let state = Arc::new(Self::new(
            auth,
            rooms,
            SrsMediaProvider::from_env(),
            wallet,
            chat,
            ChatRateLimiter::default(),
            centrifugo,
            publisher_from_env(),
            moderation,
            social,
            reports,
            DeletedUsers::new(),
            profile_extras,
            db,
        ));
        state.wallet.seed_default_gifts().await;
        Ok(state)
    }
}
