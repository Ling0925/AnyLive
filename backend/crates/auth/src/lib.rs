//! Auth domain: email OTP, JWT access/refresh, in-memory stores for offline tests.
//!
//! Structure is trait-first so SQLx/Redis adapters can replace memory later.
//!
//! # Public surface for the API crate
//! - [`OtpService`] — generate/store/verify 6-digit codes (5 min TTL, attempt limits)
//! - [`JwtService`] / [`TokenService`] — HS256 access (15m) + refresh tokens
//! - [`InMemoryUserStore`] + [`InMemoryRefreshStore`] — offline-capable stores
//! - [`AuthService`] / [`MemoryAuthService`] — passwordless OTP login facade
//!
//! Secrets: `JWT_ACCESS_SECRET`, `JWT_REFRESH_SECRET` (see [`JwtConfig::from_env`]).

mod jwt;
mod otp;
mod service;
mod store;

pub use jwt::{
    AccessClaims, IssuedTokens, JwtConfig, JwtService, RefreshClaims, TokenPair, ACCESS_TTL_SECS,
    REFRESH_TTL_SECS,
};
/// Alias matching the architecture name `TokenService`.
pub use jwt::JwtService as TokenService;
pub use otp::{normalize_email, OtpCode, OtpConfig, OtpService};
pub use service::{
    AuthService, AuthSession, LogoutRequest, MemoryAuthService, OtpSendRequest, OtpVerifyRequest,
    RefreshRequest,
};
pub use store::{
    InMemoryOtpStore, InMemoryRefreshStore, InMemoryUserStore, OtpChallenge, OtpStore, RefreshStore,
    UserStore,
};

/// Dev-mode fixed OTP accepted when no pending challenge exists (or always in tests).
pub const DEV_OTP_CODE: &str = "123456";
