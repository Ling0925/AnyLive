//! Auth application service: OTP login, refresh, logout.

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{User, UserId};
use serde::{Deserialize, Serialize};

use crate::jwt::{JwtService, TokenPair};
use crate::notifier::SharedOtpNotifier;
use crate::otp::{normalize_email, OtpService};
use crate::store::{OtpStore, RefreshStore, UserStore};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OtpSendRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OtpVerifyRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogoutRequest {
    /// Optional refresh token to revoke specifically.
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub user: User,
    pub tokens: TokenPair,
}

#[derive(Clone)]
pub struct AuthService<U, O, R>
where
    U: UserStore,
    O: OtpStore,
    R: RefreshStore,
{
    users: U,
    otp: OtpService<O>,
    refresh: R,
    jwt: JwtService,
    notifier: SharedOtpNotifier,
}

impl<U, O, R> AuthService<U, O, R>
where
    U: UserStore,
    O: OtpStore,
    R: RefreshStore,
{
    pub fn new(users: U, otp: OtpService<O>, refresh: R, jwt: JwtService) -> Self {
        Self::with_notifier(
            users,
            otp,
            refresh,
            jwt,
            Arc::new(crate::notifier::LogOtpNotifier),
        )
    }

    pub fn with_notifier(
        users: U,
        otp: OtpService<O>,
        refresh: R,
        jwt: JwtService,
        notifier: SharedOtpNotifier,
    ) -> Self {
        Self {
            users,
            otp,
            refresh,
            jwt,
            notifier,
        }
    }

    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }

    pub fn users(&self) -> &U {
        &self.users
    }

    pub fn refresh_store(&self) -> &R {
        &self.refresh
    }

    pub fn otp_config(&self) -> &crate::otp::OtpConfig {
        self.otp.config()
    }

    /// Issue OTP, store its hash, and deliver the plaintext code via notifier.
    pub async fn send_otp(&self, req: OtpSendRequest) -> Result<(), AppError> {
        let code = self.otp.send(&req.email).await?;
        self.notifier.send_otp(&req.email, &code).await?;
        Ok(())
    }

    pub async fn verify_otp(&self, req: OtpVerifyRequest) -> Result<AuthSession, AppError> {
        let email = self.otp.verify(&req.email, &req.code).await?;
        let user = self.users.upsert_by_email(&email).await?;
        // Issue tokens only after user load so callers can still reject deleted/banned
        // before insert — but prefer checking at the route layer *after* user is known
        // and *before* refresh insert when possible. We issue here; routes that need
        // ban/delete checks must run them before calling this, or revoke on reject.
        // For the common path, routes check after and we add hooks below.
        self.issue_session(user).await
    }

    /// Mint a session for an already-resolved user (OAuth / SSO paths).
    pub async fn issue_session(&self, user: User) -> Result<AuthSession, AppError> {
        let issued = self.jwt.issue_pair(user.id, user.email.clone())?;
        self.refresh
            .insert(issued.refresh_jti, issued.user_id, issued.refresh_exp)
            .await?;
        Ok(AuthSession {
            user,
            tokens: issued.pair,
        })
    }

    /// Upsert by email and mint a session (used by OAuth after identity resolve).
    pub async fn login_by_email(&self, email: &str) -> Result<AuthSession, AppError> {
        let user = self.users.upsert_by_email(email).await?;
        self.issue_session(user).await
    }

    /// Like [`login_by_email`] with a gate after user upsert / before tokens.
    pub async fn login_by_email_gated<F, Fut>(
        &self,
        email: &str,
        gate: F,
    ) -> Result<AuthSession, AppError>
    where
        F: FnOnce(User) -> Fut,
        Fut: std::future::Future<Output = Result<User, AppError>>,
    {
        let user = self.users.upsert_by_email(email).await?;
        let user = gate(user).await?;
        self.issue_session(user).await
    }

    /// Like [`verify_otp`] but runs `gate` after user upsert and **before**
    /// issuing tokens. Used by the API to reject deleted/banned accounts without
    /// orphan refresh rows.
    pub async fn verify_otp_gated<F, Fut>(
        &self,
        req: OtpVerifyRequest,
        gate: F,
    ) -> Result<AuthSession, AppError>
    where
        F: FnOnce(User) -> Fut,
        Fut: std::future::Future<Output = Result<User, AppError>>,
    {
        let email = self.otp.verify(&req.email, &req.code).await?;
        let user = self.users.upsert_by_email(&email).await?;
        let user = gate(user).await?;
        self.issue_session(user).await
    }

    /// Rotate refresh: insert the new jti first, then revoke the old one.
    /// If user lookup / issue fails, the old token stays active so the client
    /// can retry. Concurrent double-refresh still fails the second revoke path
    /// only after both have verified JWT — residual race is acceptable vs
    /// losing the only valid refresh on a mid-path error.
    pub async fn refresh(&self, req: RefreshRequest) -> Result<TokenPair, AppError> {
        let claims = self.jwt.verify_refresh(&req.refresh_token)?;
        // Confirm still active before rotating (soft check; race handled by revoke).
        if !self.refresh.is_active(claims.jti).await? {
            return Err(AppError::new(
                ErrorCode::AuthTokenRevoked,
                "refresh token revoked",
            ));
        }
        let user = self
            .users
            .find_by_id(UserId(claims.sub))
            .await?
            .ok_or_else(|| AppError::unauthorized("user not found"))?;
        let issued = self.jwt.issue_pair(user.id, user.email.clone())?;
        // Insert new first so a failure after this still leaves the client with
        // a valid refresh (old remains until we revoke).
        self.refresh
            .insert(issued.refresh_jti, issued.user_id, issued.refresh_exp)
            .await?;
        let was_active = self.refresh.revoke(claims.jti).await?;
        if !was_active {
            // Concurrent rotation already consumed the old jti — revoke the new
            // one we just inserted to avoid leaving an extra live session.
            let _ = self.refresh.revoke(issued.refresh_jti).await;
            return Err(AppError::new(
                ErrorCode::AuthTokenRevoked,
                "refresh token revoked",
            ));
        }
        Ok(issued.pair)
    }

    /// Logout: revoke provided refresh token, or all tokens for the user.
    pub async fn logout(
        &self,
        user_id: UserId,
        refresh_token: Option<&str>,
    ) -> Result<(), AppError> {
        if let Some(token) = refresh_token {
            if let Ok(claims) = self.jwt.verify_refresh(token) {
                if claims.sub == user_id.0 {
                    let _ = self.refresh.revoke(claims.jti).await?;
                    return Ok(());
                }
            }
        }
        let _ = self.refresh.revoke_all_for_user(user_id).await?;
        Ok(())
    }

    pub async fn me(&self, user_id: UserId) -> Result<User, AppError> {
        self.users
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("user not found"))
    }

    /// Update the authenticated user's display name.
    pub async fn update_display_name(
        &self,
        user_id: UserId,
        display_name: String,
    ) -> Result<User, AppError> {
        self.users.update_display_name(user_id, display_name).await
    }

    pub async fn require_email_normalized(email: &str) -> Result<String, AppError> {
        normalize_email(email)
    }
}

/// Convenience factory for fully in-memory auth (tests + local dev without PG).
pub type MemoryAuthService = AuthService<
    crate::store::InMemoryUserStore,
    crate::store::InMemoryOtpStore,
    crate::store::InMemoryRefreshStore,
>;

impl MemoryAuthService {
    /// In-memory auth for local dev + integration tests (fixed OTP `123456`).
    pub fn memory_dev() -> Self {
        use crate::jwt::JwtConfig;
        use crate::otp::OtpConfig;
        use crate::store::{InMemoryOtpStore, InMemoryRefreshStore, InMemoryUserStore};

        let jwt = JwtService::new(JwtConfig::from_env());
        // Explicit dev OTP — never use OtpConfig::default() here (defaults secure).
        let otp = OtpService::new(InMemoryOtpStore::default(), OtpConfig::dev());
        Self::with_notifier(
            InMemoryUserStore::default(),
            otp,
            InMemoryRefreshStore::default(),
            jwt,
            Arc::new(crate::notifier::NoopOtpNotifier),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifier::OtpNotifier;
    use crate::jwt::JwtConfig;
    use crate::otp::OtpConfig;
    use crate::store::{InMemoryOtpStore, InMemoryRefreshStore, InMemoryUserStore};
    use crate::DEV_OTP_CODE;

    fn service() -> MemoryAuthService {
        let jwt = JwtService::new(JwtConfig::default());
        let otp = OtpService::new(InMemoryOtpStore::default(), OtpConfig::dev());
        AuthService::with_notifier(
            InMemoryUserStore::default(),
            otp,
            InMemoryRefreshStore::default(),
            jwt,
            Arc::new(crate::notifier::NoopOtpNotifier),
        )
    }

    #[tokio::test]
    async fn passwordless_flow_send_verify_me() {
        let svc = service();
        svc.send_otp(OtpSendRequest {
            email: "flow@example.com".into(),
        })
        .await
        .unwrap();
        let session = svc
            .verify_otp(OtpVerifyRequest {
                email: "flow@example.com".into(),
                code: DEV_OTP_CODE.into(),
            })
            .await
            .unwrap();
        assert_eq!(session.user.email.as_deref(), Some("flow@example.com"));
        assert!(!session.tokens.access_token.is_empty());
        assert_eq!(session.tokens.expires_in, 15 * 60);

        let me = svc.me(session.user.id).await.unwrap();
        assert_eq!(me.id, session.user.id);
    }

    #[tokio::test]
    async fn gated_verify_rejects_before_token_issue() {
        let svc = service();
        let err = svc
            .verify_otp_gated(
                OtpVerifyRequest {
                    email: "banned@example.com".into(),
                    code: DEV_OTP_CODE.into(),
                },
                |_user| async {
                    Err(AppError::new(
                        ErrorCode::ForbiddenPolicy,
                        "user is banned",
                    ))
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ForbiddenPolicy);
    }

    #[tokio::test]
    async fn update_display_name_via_service() {
        let svc = service();
        let session = svc
            .verify_otp(OtpVerifyRequest {
                email: "rename@example.com".into(),
                code: DEV_OTP_CODE.into(),
            })
            .await
            .unwrap();
        let updated = svc
            .update_display_name(session.user.id, "Renamed".into())
            .await
            .unwrap();
        assert_eq!(updated.display_name, "Renamed");
        let me = svc.me(session.user.id).await.unwrap();
        assert_eq!(me.display_name, "Renamed");
    }

    #[tokio::test]
    async fn refresh_rotates_and_old_revoked() {
        let svc = service();
        let session = svc
            .verify_otp(OtpVerifyRequest {
                email: "r@example.com".into(),
                code: DEV_OTP_CODE.into(),
            })
            .await
            .unwrap();
        let old_refresh = session.tokens.refresh_token.clone();
        let new_pair = svc
            .refresh(RefreshRequest {
                refresh_token: old_refresh.clone(),
            })
            .await
            .unwrap();
        assert_ne!(new_pair.refresh_token, old_refresh);
        let err = svc
            .refresh(RefreshRequest {
                refresh_token: old_refresh,
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthTokenRevoked);
    }

    #[tokio::test]
    async fn logout_revokes_refresh() {
        let svc = service();
        let session = svc
            .verify_otp(OtpVerifyRequest {
                email: "out@example.com".into(),
                code: DEV_OTP_CODE.into(),
            })
            .await
            .unwrap();
        svc.logout(session.user.id, Some(&session.tokens.refresh_token))
            .await
            .unwrap();
        let err = svc
            .refresh(RefreshRequest {
                refresh_token: session.tokens.refresh_token,
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthTokenRevoked);
    }

    #[tokio::test]
    async fn invalid_otp_rejected() {
        let svc = service();
        // Non-dev path: store a real challenge by temporarily using wrong code after send
        // With dev_fixed_otp, only wrong format / non-123456 fails when challenge exists.
        let err = svc
            .verify_otp(OtpVerifyRequest {
                email: "bad@example.com".into(),
                code: "999999".into(),
            })
            .await
            .unwrap_err();
        // In dev mode without send, only 123456 works; 999999 hits missing challenge path.
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
    }

    #[tokio::test]
    async fn refresh_reuse_rejected() {
        let svc = service();
        let session = svc
            .verify_otp(OtpVerifyRequest {
                email: "reuse@example.com".into(),
                code: DEV_OTP_CODE.into(),
            })
            .await
            .unwrap();
        let rt = session.tokens.refresh_token.clone();
        let _ = svc
            .refresh(RefreshRequest {
                refresh_token: rt.clone(),
            })
            .await
            .unwrap();
        let err = svc
            .refresh(RefreshRequest { refresh_token: rt })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthTokenRevoked);
    }

    #[tokio::test]
    async fn send_delivers_via_notifier() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct CountingNotifier {
            n: AtomicUsize,
        }
        #[async_trait::async_trait]
        impl OtpNotifier for CountingNotifier {
            async fn send_otp(&self, _email: &str, code: &str) -> Result<(), AppError> {
                assert_eq!(code, DEV_OTP_CODE);
                self.n.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }
        let jwt = JwtService::new(JwtConfig::default());
        let otp = OtpService::new(InMemoryOtpStore::default(), OtpConfig::dev());
        let counter = Arc::new(CountingNotifier {
            n: AtomicUsize::new(0),
        });
        let svc = AuthService::with_notifier(
            InMemoryUserStore::default(),
            otp,
            InMemoryRefreshStore::default(),
            jwt,
            counter.clone(),
        );
        svc.send_otp(OtpSendRequest {
            email: "n@example.com".into(),
        })
        .await
        .unwrap();
        assert_eq!(counter.n.load(Ordering::SeqCst), 1);
    }
}
