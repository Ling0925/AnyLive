//! Shared API state.

use std::sync::Arc;

use anylive_auth::{
    AuthService, InMemoryOtpStore, InMemoryRefreshStore, JwtConfig, JwtService, OtpConfig,
    OtpService,
};
use anylive_db::{postgres_enabled, AnyUserStore, PgPool};
use anylive_media::SrsMediaProvider;
use anylive_moderation::MemoryModeration;
use anylive_realtime::{
    publisher_from_env, CentrifugoConfig, CentrifugoPublisher, MemoryChatBus,
    NoopCentrifugoPublisher,
};
use anylive_social::MemorySocial;
use anylive_wallet::MemoryWallet;

use crate::guards::{check_production_secrets, is_production_env};
use crate::rooms::MemoryRoomStore;
use crate::routes::reports::MemoryReports;

/// Auth service with pluggable user store (memory default, Postgres when enabled).
pub type AppAuthService = AuthService<AnyUserStore, InMemoryOtpStore, InMemoryRefreshStore>;

#[derive(Clone)]
pub struct AppState {
    pub auth: AppAuthService,
    pub rooms: MemoryRoomStore,
    pub media: SrsMediaProvider,
    pub wallet: MemoryWallet,
    pub chat: MemoryChatBus,
    pub centrifugo: CentrifugoConfig,
    /// Centrifugo HTTP API publisher (noop when env not set).
    pub centrifugo_publisher: Arc<dyn CentrifugoPublisher>,
    pub moderation: MemoryModeration,
    pub social: MemorySocial,
    pub reports: MemoryReports,
    /// Present when `USE_POSTGRES=1` + `DATABASE_URL` were used at startup.
    pub db: Option<PgPool>,
}

impl AppState {
    pub fn new(
        auth: AppAuthService,
        rooms: MemoryRoomStore,
        media: SrsMediaProvider,
        wallet: MemoryWallet,
        chat: MemoryChatBus,
        centrifugo: CentrifugoConfig,
        centrifugo_publisher: Arc<dyn CentrifugoPublisher>,
        moderation: MemoryModeration,
        social: MemorySocial,
        reports: MemoryReports,
        db: Option<PgPool>,
    ) -> Self {
        Self {
            auth,
            rooms,
            media,
            wallet,
            chat,
            centrifugo,
            centrifugo_publisher,
            moderation,
            social,
            reports,
            db,
        }
    }

    /// Local/dev defaults (fixed OTP `123456`, insecure JWT defaults allowed).
    /// Unchanged for integration tests and offline development.
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
            MemoryRoomStore::new(),
            SrsMediaProvider::from_env(),
            MemoryWallet::new(),
            MemoryChatBus::new(),
            CentrifugoConfig::default(),
            // Tests/offline: never require a live Centrifugo.
            Arc::new(NoopCentrifugoPublisher::new()),
            MemoryModeration::new(),
            MemorySocial::new(),
            MemoryReports::new(),
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
    /// - `USE_POSTGRES=1` + `DATABASE_URL` — optional Postgres user store + migrations
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

        let (users, db) = if postgres_enabled() {
            let pool = anylive_db::connect_and_migrate_from_env()
                .await
                .map_err(|e| format!("postgres connect/migrate failed: {e}"))?;
            tracing::info!("postgres enabled: migrations applied, using PostgresUserStore");
            (AnyUserStore::postgres(pool.clone()), Some(pool))
        } else {
            tracing::info!(
                "postgres disabled (set USE_POSTGRES=1 and DATABASE_URL to enable)"
            );
            (AnyUserStore::memory(), None)
        };

        let auth = AuthService::new(users, otp, InMemoryRefreshStore::default(), jwt);

        let state = Arc::new(Self::new(
            auth,
            MemoryRoomStore::new(),
            SrsMediaProvider::from_env(),
            MemoryWallet::new(),
            MemoryChatBus::new(),
            centrifugo,
            publisher_from_env(),
            MemoryModeration::new(),
            MemorySocial::new(),
            MemoryReports::new(),
            db,
        ));
        state.wallet.seed_default_gifts().await;
        Ok(state)
    }
}
