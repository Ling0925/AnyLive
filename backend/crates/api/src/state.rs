//! Shared API state.

use std::sync::Arc;

use anylive_auth::{
    otp_notifier_from_env, AuthService, JwtConfig, JwtService, NoopOtpNotifier, OtpConfig,
    OtpService, SharedOtpNotifier,
};
use anylive_db::{
    postgres_enabled, AnyChat, AnyDeletedUsers, AnyModeration, AnyOtpStore, AnyProfileExtras,
    AnyRefreshStore, AnyReports, AnySocial, AnyPayStore, AnyUserStore, AnyWallet, PgPool,
};
use anylive_media::SrsMediaProvider;
use anylive_pay::{PayChannelRegistry, PayStore};
use anylive_realtime::{
    nats_publisher_from_env, publisher_from_env, CentrifugoConfig, CentrifugoPublisher,
    ChatRateLimiter, NatsPublisher, NoopCentrifugoPublisher, NoopNatsPublisher,
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
    /// Pay products / orders store (memory or Postgres).
    pub pay: AnyPayStore,
    /// Enabled payment channel providers.
    pub pay_registry: PayChannelRegistry,
    /// Public API base for notify_url construction.
    pub pay_public_base: String,
    /// HMAC secret for mock pay sandbox-complete (None when mock disabled).
    pub pay_mock_secret: Option<String>,
    /// Rate limit sandbox-complete minting (per user id).
    pub pay_sandbox_limiter: IpRateLimiter,
    /// Soft-launch invite / email allowlist gate (P2).
    pub invite: crate::invite::InviteGate,
    /// Chat content policy word filter (empty = open).
    pub word_filter: anylive_moderation::WordFilter,
    /// Optional LiveKit interactive provider (P3 co-host).
    pub livekit: Option<anylive_media::LiveKitProvider>,
    /// Co-host invites + PK sessions (in-memory control plane).
    pub interactive: crate::interactive::InteractiveStore,
    /// Client analytics event buffer (P4 scaffold).
    pub analytics: crate::analytics::AnalyticsStore,
    /// Room online presence + likes (WBS E4.4, in-process).
    pub presence: crate::presence::PresenceStore,
    /// Room recording enable flags (WBS E3.5 control plane).
    pub recording: crate::recording::RecordingStore,
    /// Device push tokens (WBS E8.9 scaffold, in-process; no delivery).
    pub push: crate::push::PushStore,
    /// Push delivery backend (noop / log / http).
    pub push_delivery: crate::push_delivery::SharedPushDelivery,
    /// OAuth exchange config (stub / providers).
    pub oauth: crate::oauth::OauthConfig,
    /// Optional object storage for avatars (WBS E2.3).
    pub object_storage: crate::object_storage::ObjectStorageConfig,
    /// Optional NATS domain-event publisher (WBS E1.3 / E5.3).
    pub nats: Arc<dyn NatsPublisher>,
    /// GA / soft-launch kill switches.
    pub features: crate::features::FeatureFlags,
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
        pay: AnyPayStore,
        pay_registry: PayChannelRegistry,
        pay_public_base: String,
        pay_mock_secret: Option<String>,
        pay_sandbox_limiter: IpRateLimiter,
        invite: crate::invite::InviteGate,
        word_filter: anylive_moderation::WordFilter,
        livekit: Option<anylive_media::LiveKitProvider>,
        interactive: crate::interactive::InteractiveStore,
        analytics: crate::analytics::AnalyticsStore,
        features: crate::features::FeatureFlags,
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
            pay,
            pay_registry,
            pay_public_base,
            pay_mock_secret,
            pay_sandbox_limiter,
            invite,
            word_filter,
            livekit,
            interactive,
            analytics,
            presence: crate::presence::PresenceStore::new(),
            recording: crate::recording::RecordingStore::new(),
            push: crate::push::PushStore::new(),
            // For tests: deterministic defaults (not from env).
            push_delivery: std::sync::Arc::new(crate::push_delivery::NoopPushDelivery),
            oauth: crate::oauth::OauthConfig {
                stub_enabled: true,
                enabled: crate::oauth::OAUTH_PROVIDERS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            },
            object_storage: crate::object_storage::ObjectStorageConfig::dev(),
            nats: Arc::new(NoopNatsPublisher::new()),
            features,
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
            AnyPayStore::memory(),
            PayChannelRegistry::sandbox_channels("anylive-dev-pay-mock-secret-change-me"),
            "http://localhost:8088".into(),
            Some("anylive-dev-pay-mock-secret-change-me".into()),
            // High limit for tests; production forbids mock entirely.
            IpRateLimiter::new(10_000, std::time::Duration::from_secs(60)),
            crate::invite::InviteGate::open(),
            anylive_moderation::WordFilter::empty(),
            None,
            crate::interactive::InteractiveStore::new(),
            crate::analytics::AnalyticsStore::new(),
            crate::features::FeatureFlags::all_enabled(),
        ))
    }

    /// Async init for tests/dev that need gift catalog.
    pub async fn dev_ready() -> Arc<Self> {
        let state = Self::dev();
        state.wallet.seed_default_gifts().await;
        state.pay.seed_default_products().await;
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
            pay_store,
            db,
        ) = if postgres_enabled() {
            let pool = anylive_db::connect_and_migrate_from_env()
                .await
                .map_err(|e| format!("postgres connect/migrate failed: {e}"))?;
            tracing::info!(
                "postgres enabled: migrations applied; using Postgres dual stores for \
                 users/rooms/wallet/social/moderation/reports/chat/profile_extras/\
                 deleted_users/refresh/otp/pay"
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
                AnyPayStore::postgres(pool.clone()),
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
                AnyPayStore::memory(),
                None,
            )
        };

        let otp = OtpService::new(otp_store, otp_cfg);
        let auth = AuthService::with_notifier(users, otp, refresh, jwt, notifier);
        let allow_mock_topup = mock_topup_allowed();
        if allow_mock_topup {
            tracing::warn!("ALLOW_MOCK_TOPUP=1: mock wallet topup is enabled");
        }

        let pay_registry = PayChannelRegistry::from_env();
        let pay_registry_has_mock = pay_registry
            .get(anylive_pay::PayChannel::Mock)
            .is_some();
        let pay_public_base = std::env::var("PAY_PUBLIC_BASE_URL")
            .or_else(|_| std::env::var("API_PUBLIC_BASE_URL"))
            .unwrap_or_else(|_| "http://localhost:8088".into());
        if pay_registry.enabled_channels().is_empty() {
            tracing::info!("no pay channels enabled (set PAY_CHANNELS or PAY_ENABLE_MOCK=1)");
        } else {
            tracing::info!(
                channels = ?pay_registry
                    .enabled_channels()
                    .iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>(),
                "pay channels enabled"
            );
        }

        let pay_mock_secret = if pay_registry_has_mock {
            let secret = std::env::var("PAY_MOCK_SECRET").unwrap_or_else(|_| {
                anylive_pay::DEFAULT_PAY_MOCK_SECRET.to_string()
            });
            // Default secret is public in source; only allow it for local/dev/test.
            if !is_local_env(&app_env) && secret == anylive_pay::DEFAULT_PAY_MOCK_SECRET {
                return Err(
                    "mock pay requires a non-default PAY_MOCK_SECRET outside local/dev/test"
                        .into(),
                );
            }
            if secret == anylive_pay::DEFAULT_PAY_MOCK_SECRET {
                tracing::warn!(
                    "PAY_MOCK_SECRET is the built-in default; forgeable on open networks — set a unique secret for shared dogfood"
                );
            }
            Some(secret)
        } else {
            None
        };

        let mut state = Self::new(
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
            pay_store,
            pay_registry,
            pay_public_base,
            pay_mock_secret,
            // Dogfood mint guard: 10 sandbox completes / user / hour.
            IpRateLimiter::new(10, std::time::Duration::from_secs(3600)),
            crate::invite::InviteGate::from_env(),
            anylive_moderation::WordFilter::from_env(),
            anylive_media::LiveKitProvider::from_env(),
            crate::interactive::InteractiveStore::new(),
            crate::analytics::AnalyticsStore::new(),
            crate::features::FeatureFlags::from_env(),
        );
        // Env-backed object storage + NATS (dev() defaults used by tests).
        state.object_storage = crate::object_storage::ObjectStorageConfig::from_env();
        state.nats = nats_publisher_from_env();
        state.push_delivery = crate::push_delivery::push_delivery_from_env();
        state.oauth = crate::oauth::OauthConfig::from_env();
        if state.oauth.stub_enabled {
            tracing::warn!("oauth stub mode enabled (OAUTH_STUB / local APP_ENV)");
        }
        let state = Arc::new(state);
        state.wallet.seed_default_gifts().await;
        state.pay.seed_default_products().await;
        Ok(state)
    }
}
