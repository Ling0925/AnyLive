//! Shared API state.

use std::sync::Arc;

use anylive_auth::{
    otp_notifier_from_env, AuthService, JwtConfig, JwtService, NoopOtpNotifier, OtpConfig,
    OtpService, SharedOtpNotifier,
};
use anylive_db::{
    postgres_enabled, AnyChat, AnyDeletedUsers, AnyModeration, AnyOtpStore, AnyProfileExtras,
    AnyRefreshStore, AnyReports, AnySocial, AnyUserStore, AnyWallet, PgPool,
};
use anylive_media::SrsMediaProvider;
use anylive_realtime::{
    publisher_from_env, CentrifugoConfig, CentrifugoPublisher, ChatRateLimiter,
    NoopCentrifugoPublisher,
};

use crate::guards::{
    check_production_secrets_ext, is_local_env, is_production_env, mock_topup_allowed,
};
use crate::rate_limit::IpRateLimiter;
use crate::rooms::AnyRoomStore;

/// Auth service with pluggable user + OTP + refresh stores (memory default, Postgres when enabled).
pub type AppAuthService = AuthService<AnyUserStore, AnyOtpStore, AnyRefreshStore>;

#[derive(Clone)]
pub struct AppState {
    pub auth: AppAuthService,
    pub rooms: AnyRoomStore,
    pub media: SrsMediaProvider,
    pub wallet: AnyWallet,
    pub chat: AnyChat,
    /// Per-user chat post rate limiter (in-memory sliding window).
    pub chat_rate_limiter: ChatRateLimiter,
    /// Per-IP rate limiter for unauthenticated OTP endpoints.
    pub otp_ip_limiter: IpRateLimiter,
    pub centrifugo: CentrifugoConfig,
    /// Centrifugo HTTP API publisher (noop when env not set).
    pub centrifugo_publisher: Arc<dyn CentrifugoPublisher>,
    pub moderation: AnyModeration,
    pub social: AnySocial,
    pub reports: AnyReports,
    /// Soft-deleted accounts (memory dual store; Postgres when USE_POSTGRES=1).
    pub deleted_users: AnyDeletedUsers,
    /// Age/privacy declarations (memory dual store; Postgres when USE_POSTGRES=1).
    pub profile_extras: AnyProfileExtras,
    /// Present when `USE_POSTGRES=1` + `DATABASE_URL` were used at startup.
    pub db: Option<PgPool>,
    /// Whether mock topup is enabled for this process (`ALLOW_MOCK_TOPUP=1`).
    pub allow_mock_topup: bool,
}

impl AppState {
    pub fn new(
        auth: AppAuthService,
        rooms: AnyRoomStore,
        media: SrsMediaProvider,
        wallet: AnyWallet,
        chat: AnyChat,
        chat_rate_limiter: ChatRateLimiter,
        otp_ip_limiter: IpRateLimiter,
        centrifugo: CentrifugoConfig,
        centrifugo_publisher: Arc<dyn CentrifugoPublisher>,
        moderation: AnyModeration,
        social: AnySocial,
        reports: AnyReports,
        deleted_users: AnyDeletedUsers,
        profile_extras: AnyProfileExtras,
        db: Option<PgPool>,
        allow_mock_topup: bool,
    ) -> Self {
        Self {
            auth,
            rooms,
            media,
            wallet,
            chat,
            chat_rate_limiter,
            otp_ip_limiter,
            centrifugo,
            centrifugo_publisher,
            moderation,
            social,
            reports,
            deleted_users,
            profile_extras,
            db,
            allow_mock_topup,
        }
    }

    /// Local/dev defaults (fixed OTP `123456`, insecure JWT defaults allowed).
    /// Unchanged for integration tests and offline development — always memory stores.
    pub fn dev() -> Arc<Self> {
        let jwt = JwtService::new(JwtConfig::default());
        let otp = OtpService::new(AnyOtpStore::memory(), OtpConfig::dev());
        let auth = AuthService::with_notifier(
            AnyUserStore::memory(),
            otp,
            AnyRefreshStore::memory(),
            jwt,
            Arc::new(NoopOtpNotifier) as SharedOtpNotifier,
        );
        Arc::new(Self::new(
            auth,
            AnyRoomStore::memory(),
            SrsMediaProvider::from_env(),
            AnyWallet::memory(),
            AnyChat::memory(),
            ChatRateLimiter::default(),
            // High limit for tests so OTP send is not flaky under suite load.
            IpRateLimiter::new(10_000, std::time::Duration::from_secs(60)),
            CentrifugoConfig::default(),
            // Tests/offline: never require a live Centrifugo.
            Arc::new(NoopCentrifugoPublisher::new()),
            AnyModeration::memory(),
            AnySocial::memory(),
            AnyReports::memory(),
            AnyDeletedUsers::memory(),
            AnyProfileExtras::memory(),
            None,
            true, // tests exercise mock topup
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
    /// - `ALLOW_DEV_OTP=1` — explicit fixed OTP (never implied by non-prod APP_ENV)
    /// - `ALLOW_MOCK_TOPUP=1` — explicit mock wallet topup
    /// - `OTP_NOTIFIER` — delivery backend (`log` / `smtp` / …); required in production
    /// - `USE_POSTGRES=1` + `DATABASE_URL` — optional Postgres dual stores + migrations
    ///
    /// Realtime routes are always mounted, so Centrifugo secret is always guarded
    /// in production.
    pub async fn from_env() -> Result<Arc<Self>, String> {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into());
        let jwt_cfg = JwtConfig::from_env();
        let centrifugo = CentrifugoConfig::default();
        // Fixed OTP only with explicit ALLOW_DEV_OTP — never "not production".
        let otp_cfg = OtpConfig::from_env();
        if otp_cfg.dev_fixed_otp && is_production_env(&app_env) {
            return Err("ALLOW_DEV_OTP is forbidden in production".into());
        }
        // Local ergonomics: if neither ALLOW_DEV_OTP nor a notifier is set, default
        // to fixed OTP for offline dogfood. Staging/dogfood without the flag gets
        // secure random OTP and must configure OTP_NOTIFIER.
        let otp_cfg = if !otp_cfg.dev_fixed_otp
            && is_local_env(&app_env)
            && std::env::var("OTP_NOTIFIER").is_err()
            && !is_production_env(&app_env)
        {
            tracing::warn!(
                "local APP_ENV without OTP_NOTIFIER: enabling fixed OTP (set ALLOW_DEV_OTP=0 and OTP_NOTIFIER to disable)"
            );
            // Keep secure default for staging; only auto-dev for local/dev/test.
            OtpConfig::dev()
        } else {
            otp_cfg
        };

        // Realtime token endpoint is always present in this binary.
        const REALTIME_USED: bool = true;
        let srs_webhook = std::env::var("SRS_WEBHOOK_SECRET").ok();
        let otp_notifier_kind = std::env::var("OTP_NOTIFIER").unwrap_or_default();
        check_production_secrets_ext(
            &app_env,
            &jwt_cfg.access_secret,
            &jwt_cfg.refresh_secret,
            otp_cfg.dev_fixed_otp,
            Some(&centrifugo.token_secret),
            REALTIME_USED,
            srs_webhook.as_deref(),
            Some(otp_notifier_kind.as_str()),
        )?;

        let jwt = JwtService::new(jwt_cfg);
        let notifier = if otp_cfg.dev_fixed_otp {
            // Fixed OTP never needs delivery of a random code.
            Arc::new(NoopOtpNotifier) as SharedOtpNotifier
        } else {
            otp_notifier_from_env()
        };

        let (
            users,
            rooms,
            wallet,
            moderation,
            social,
            reports,
            chat,
            profile_extras,
            deleted_users,
            refresh,
            otp_store,
            db,
        ) = if postgres_enabled() {
            let pool = anylive_db::connect_and_migrate_from_env()
                .await
                .map_err(|e| format!("postgres connect/migrate failed: {e}"))?;
            tracing::info!(
                "postgres enabled: migrations applied; using Postgres dual stores for \
                 users/rooms/wallet/social/moderation/reports/chat/profile_extras/\
                 deleted_users/refresh/otp"
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
                AnyDeletedUsers::postgres(pool.clone()),
                AnyRefreshStore::postgres(pool.clone()),
                AnyOtpStore::postgres(pool.clone()),
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
            tracing::info!("postgres disabled (set USE_POSTGRES=1 and DATABASE_URL to enable)");
            (
                AnyUserStore::memory(),
                AnyRoomStore::memory(),
                AnyWallet::memory(),
                AnyModeration::memory(),
                AnySocial::memory(),
                AnyReports::memory(),
                AnyChat::memory(),
                AnyProfileExtras::memory(),
                AnyDeletedUsers::memory(),
                AnyRefreshStore::memory(),
                AnyOtpStore::memory(),
                None,
            )
        };

        let otp = OtpService::new(otp_store, otp_cfg);
        let auth = AuthService::with_notifier(users, otp, refresh, jwt, notifier);
        let allow_mock_topup = mock_topup_allowed();
        if allow_mock_topup {
            tracing::warn!("ALLOW_MOCK_TOPUP=1: mock wallet topup is enabled");
        }

        let state = Arc::new(Self::new(
            auth,
            rooms,
            SrsMediaProvider::from_env(),
            wallet,
            chat,
            ChatRateLimiter::default(),
            IpRateLimiter::default(),
            centrifugo,
            publisher_from_env(),
            moderation,
            social,
            reports,
            deleted_users,
            profile_extras,
            db,
            allow_mock_topup,
        ));
        state.wallet.seed_default_gifts().await;
        Ok(state)
    }
}
