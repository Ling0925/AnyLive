//! Auth application service: OTP login, refresh, logout.

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{User, UserId};
use serde::{Deserialize, Serialize};

use crate::jwt::{JwtService, TokenPair};
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
}

impl<U, O, R> AuthService<U, O, R>
where
    U: UserStore,
    O: OtpStore,
    R: RefreshStore,
{
    pub fn new(users: U, otp: OtpService<O>, refresh: R, jwt: JwtService) -> Self {
        Self {
            users,
            otp,
            refresh,
            jwt,
        }
    }

    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }

    pub fn users(&self) -> &U {
        &self.users
    }

    pub async fn send_otp(&self, req: OtpSendRequest) -> Result<(), AppError> {
        let _ = self.otp.send(&req.email).await?;
        Ok(())
    }

    pub async fn verify_otp(&self, req: OtpVerifyRequest) -> Result<AuthSession, AppError> {
        let email = self.otp.verify(&req.email, &req.code).await?;
        let user = self.users.upsert_by_email(&email).await?;
        let issued = self.jwt.issue_pair(user.id, user.email.clone())?;
        self.refresh
            .insert(issued.refresh_jti, issued.user_id, issued.refresh_exp)
            .await?;
        Ok(AuthSession {
            user,
            tokens: issued.pair,
        })
    }

    pub async fn refresh(&self, req: RefreshRequest) -> Result<TokenPair, AppError> {
        let claims = self.jwt.verify_refresh(&req.refresh_token)?;
        // Atomic rotate: revoke-first so concurrent refresh of the same jti
        // cannot both succeed (second revoke returns false → treated as reused).
        let was_active = self.refresh.revoke(claims.jti).await?;
        if !was_active {
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
        self.refresh
            .insert(issued.refresh_jti, issued.user_id, issued.refresh_exp)
            .await?;
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
        Self::new(
            InMemoryUserStore::default(),
            otp,
            InMemoryRefreshStore::default(),
            jwt,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::JwtConfig;
    use crate::otp::OtpConfig;
    use crate::store::{InMemoryOtpStore, InMemoryRefreshStore, InMemoryUserStore};
    use crate::DEV_OTP_CODE;

    fn service() -> MemoryAuthService {
        let jwt = JwtService::new(JwtConfig::default());
        let otp = OtpService::new(InMemoryOtpStore::default(), OtpConfig::dev());
        AuthService::new(
            InMemoryUserStore::default(),
            otp,
            InMemoryRefreshStore::default(),
            jwt,
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
}
