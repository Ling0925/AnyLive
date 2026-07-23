//! Auth domain: email OTP, password credentials, JWT access/refresh, in-memory stores.
//!
//! Structure is trait-first so SQLx/Redis adapters can replace memory later.
//!
//! # Public surface for the API crate
//! - [`OtpService`] — generate/store/verify 6-digit codes (5 min TTL, attempt limits)
//! - [`JwtService`] / [`TokenService`] — HS256 access (15m) + refresh tokens
//! - [`InMemoryUserStore`] + [`InMemoryRefreshStore`] — offline-capable stores
//! - [`AuthService`] / [`MemoryAuthService`] — OTP + password login facade
//! - [`OtpNotifier`] — delivery port (must be wired in production)
//! - [`password`] — argon2id hash/verify + lockout policy
//!
//! Secrets: `JWT_ACCESS_SECRET`, `JWT_REFRESH_SECRET` (see [`JwtConfig::from_env`]).

mod credentials;
mod jwt;
mod notifier;
mod otp;
pub mod password;
mod service;
mod store;

pub use credentials::{CredentialRecord, CredentialStore, InMemoryCredentialStore};
pub use jwt::{
    AccessClaims, IssuedTokens, JwtConfig, JwtService, RefreshClaims, TokenPair, ACCESS_TTL_SECS,
    REFRESH_TTL_SECS,
};
/// Alias matching the architecture name `TokenService`.
pub use jwt::JwtService as TokenService;
pub use notifier::{
    otp_notifier_from_env, HttpOtpNotifier, LogOtpNotifier, NoopOtpNotifier, OtpNotifier,
    SharedOtpNotifier, UnconfiguredOtpNotifier,
};
pub use otp::{
    env_flag_enabled, hash_otp_code, normalize_email, OtpCode, OtpConfig, OtpService,
    OTP_MAX_ATTEMPTS, OTP_RESEND_COOLDOWN_SECS,
};
pub use password::{
    account_locked, generate_temp_password, hash_password, invalid_credentials, verify_password,
    PasswordPolicy, DEFAULT_LOGIN_LOCK_SECS, DEFAULT_LOGIN_MAX_ATTEMPTS, DEFAULT_PASSWORD_MIN_LEN,
};
pub use service::{
    AuthService, AuthSession, ChangePasswordRequest, LogoutRequest, MemoryAuthService,
    OtpSendRequest, OtpVerifyRequest, PasswordLoginRequest, RefreshRequest, SetPasswordResult,
};
pub use store::{
    InMemoryOtpStore, InMemoryRefreshStore, InMemoryUserStore, OtpChallenge, OtpStore,
    RefreshSessionInfo, RefreshStore, UserStore,
};

/// Dev-mode fixed OTP accepted when no pending challenge exists (or always in tests).
pub const DEV_OTP_CODE: &str = "123456";
