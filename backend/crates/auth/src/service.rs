//! Auth application service: OTP login, password login, refresh, logout.

use std::sync::Arc;

use anylive_common::{AppError, ErrorCode};
use anylive_domain::{User, UserId, UserStatus};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::credentials::{CredentialRecord, CredentialStore};
use crate::jwt::{JwtService, TokenPair};
use crate::notifier::SharedOtpNotifier;
use crate::otp::{normalize_email, OtpService};
use crate::password::{
    account_locked, generate_temp_password, hash_password, invalid_credentials, verify_password,
    PasswordPolicy,
};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PasswordLoginRequest {
    /// Email or username.
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub user: User,
    pub tokens: TokenPair,
    /// Present when password auth is configured for the user.
    #[serde(default)]
    pub must_change_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPasswordResult {
    /// Plaintext temp password only when generated server-side (once).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_password: Option<String>,
    pub must_change_password: bool,
}

#[derive(Clone)]
pub struct AuthService<U, O, R, C>
where
    U: UserStore,
    O: OtpStore,
    R: RefreshStore,
    C: CredentialStore,
{
    users: U,
    otp: OtpService<O>,
    refresh: R,
    credentials: C,
    jwt: JwtService,
    notifier: SharedOtpNotifier,
    password_policy: PasswordPolicy,
}

impl<U, O, R, C> AuthService<U, O, R, C>
where
    U: UserStore,
    O: OtpStore,
    R: RefreshStore,
    C: CredentialStore,
{
    pub fn new(users: U, otp: OtpService<O>, refresh: R, credentials: C, jwt: JwtService) -> Self {
        Self::with_notifier(
            users,
            otp,
            refresh,
            credentials,
            jwt,
            Arc::new(crate::notifier::LogOtpNotifier),
        )
    }

    pub fn with_notifier(
        users: U,
        otp: OtpService<O>,
        refresh: R,
        credentials: C,
        jwt: JwtService,
        notifier: SharedOtpNotifier,
    ) -> Self {
        Self {
            users,
            otp,
            refresh,
            credentials,
            jwt,
            notifier,
            password_policy: PasswordPolicy::default(),
        }
    }

    pub fn with_password_policy(mut self, policy: PasswordPolicy) -> Self {
        self.password_policy = policy;
        self
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

    pub fn credentials(&self) -> &C {
        &self.credentials
    }

    pub fn otp_config(&self) -> &crate::otp::OtpConfig {
        self.otp.config()
    }

    pub fn password_policy(&self) -> &PasswordPolicy {
        &self.password_policy
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
        self.issue_session(user).await
    }

    /// Mint a session for an already-resolved user (OAuth / SSO paths).
    pub async fn issue_session(&self, user: User) -> Result<AuthSession, AppError> {
        let must_change = self
            .credentials
            .get(user.id)
            .await?
            .map(|c| c.must_change_password)
            .unwrap_or(false);
        let issued = self.jwt.issue_pair(user.id, user.email.clone())?;
        self.refresh
            .insert(issued.refresh_jti, issued.user_id, issued.refresh_exp)
            .await?;
        Ok(AuthSession {
            user,
            tokens: issued.pair,
            must_change_password: must_change,
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

    /// OTP verify for an **existing** user only (no upsert). Used when public register is off.
    pub async fn verify_otp_existing_gated<F, Fut>(
        &self,
        req: OtpVerifyRequest,
        gate: F,
    ) -> Result<AuthSession, AppError>
    where
        F: FnOnce(User) -> Fut,
        Fut: std::future::Future<Output = Result<User, AppError>>,
    {
        let email = self.otp.verify(&req.email, &req.code).await?;
        let user = self
            .users
            .find_by_email(&email)
            .await?
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ForbiddenPolicy,
                    "account does not exist; contact an administrator",
                )
            })?;
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

    /// Password login by email or username.
    pub async fn password_login(
        &self,
        req: PasswordLoginRequest,
    ) -> Result<AuthSession, AppError> {
        let identifier = req.identifier.trim();
        if identifier.is_empty() || req.password.is_empty() {
            return Err(invalid_credentials());
        }
        let user = if identifier.contains('@') {
            let email = normalize_email(identifier)?;
            self.users.find_by_email(&email).await?
        } else {
            self.users.find_by_username(identifier).await?
        };
        let Some(user) = user else {
            // Dummy hash work would help timing; keep simple for Wave A.
            return Err(invalid_credentials());
        };
        if !user.status.can_login() {
            return Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "account is not active",
            ));
        }
        let mut cred = self
            .credentials
            .get(user.id)
            .await?
            .ok_or_else(invalid_credentials)?;
        if let Some(until) = cred.locked_until {
            if until > Utc::now() {
                return Err(account_locked());
            }
        }
        let ok = verify_password(&req.password, &cred.password_hash)?;
        if !ok {
            cred.failed_attempts = cred.failed_attempts.saturating_add(1);
            if cred.failed_attempts >= self.password_policy.max_attempts {
                cred.locked_until =
                    Some(Utc::now() + Duration::seconds(self.password_policy.lock_secs));
                cred.failed_attempts = 0;
            }
            self.credentials.upsert(cred).await?;
            return Err(invalid_credentials());
        }
        // Success: clear lockout counters.
        cred.failed_attempts = 0;
        cred.locked_until = None;
        let must_change = cred.must_change_password;
        self.credentials.upsert(cred).await?;
        let mut session = self.issue_session(user).await?;
        session.must_change_password = must_change;
        Ok(session)
    }

    /// Password login with an external gate (ban/delete checks) after credential success.
    pub async fn password_login_gated<F, Fut>(
        &self,
        req: PasswordLoginRequest,
        gate: F,
    ) -> Result<AuthSession, AppError>
    where
        F: FnOnce(User) -> Fut,
        Fut: std::future::Future<Output = Result<User, AppError>>,
    {
        // Resolve + verify password first (reuse core), then gate, then re-issue if needed.
        // Simpler path: login then re-check — but tokens already issued. So inline:
        let identifier = req.identifier.trim();
        if identifier.is_empty() || req.password.is_empty() {
            return Err(invalid_credentials());
        }
        let user = if identifier.contains('@') {
            let email = normalize_email(identifier)?;
            self.users.find_by_email(&email).await?
        } else {
            self.users.find_by_username(identifier).await?
        };
        let Some(user) = user else {
            return Err(invalid_credentials());
        };
        if !user.status.can_login() {
            return Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "account is not active",
            ));
        }
        let mut cred = self
            .credentials
            .get(user.id)
            .await?
            .ok_or_else(invalid_credentials)?;
        if let Some(until) = cred.locked_until {
            if until > Utc::now() {
                return Err(account_locked());
            }
        }
        let ok = verify_password(&req.password, &cred.password_hash)?;
        if !ok {
            cred.failed_attempts = cred.failed_attempts.saturating_add(1);
            if cred.failed_attempts >= self.password_policy.max_attempts {
                cred.locked_until =
                    Some(Utc::now() + Duration::seconds(self.password_policy.lock_secs));
                cred.failed_attempts = 0;
            }
            self.credentials.upsert(cred).await?;
            return Err(invalid_credentials());
        }
        cred.failed_attempts = 0;
        cred.locked_until = None;
        let must_change = cred.must_change_password;
        self.credentials.upsert(cred).await?;
        let user = gate(user).await?;
        let mut session = self.issue_session(user).await?;
        session.must_change_password = must_change;
        Ok(session)
    }

    /// Set / replace password for a user (admin provision or reset).
    /// When `password` is None, generates a temporary password.
    pub async fn set_password(
        &self,
        user_id: UserId,
        password: Option<&str>,
        must_change: bool,
    ) -> Result<SetPasswordResult, AppError> {
        let (plain, generated) = match password {
            Some(p) => {
                self.password_policy.validate_password(p)?;
                (p.to_string(), false)
            }
            None => (generate_temp_password(), true),
        };
        let hash = hash_password(&plain)?;
        self.credentials
            .upsert(CredentialRecord {
                user_id,
                password_hash: hash,
                password_updated_at: Utc::now(),
                must_change_password: must_change || generated,
                failed_attempts: 0,
                locked_until: None,
            })
            .await?;
        // Revoke all sessions on password set/reset.
        let _ = self.refresh.revoke_all_for_user(user_id).await?;
        Ok(SetPasswordResult {
            temporary_password: if generated { Some(plain) } else { None },
            must_change_password: must_change || generated,
        })
    }

    /// Authenticated user changes their own password.
    pub async fn change_password(
        &self,
        user_id: UserId,
        req: ChangePasswordRequest,
    ) -> Result<(), AppError> {
        self.password_policy
            .validate_password(&req.new_password)?;
        let mut cred = self
            .credentials
            .get(user_id)
            .await?
            .ok_or_else(|| AppError::validation("password login is not configured for this account"))?;
        if !verify_password(&req.current_password, &cred.password_hash)? {
            return Err(invalid_credentials());
        }
        cred.password_hash = hash_password(&req.new_password)?;
        cred.password_updated_at = Utc::now();
        cred.must_change_password = false;
        cred.failed_attempts = 0;
        cred.locked_until = None;
        self.credentials.upsert(cred).await?;
        let _ = self.refresh.revoke_all_for_user(user_id).await?;
        Ok(())
    }

    /// Admin-create user + password in one call.
    pub async fn provision_user(
        &self,
        display_name: String,
        email: Option<String>,
        username: Option<String>,
        password: Option<&str>,
        must_change: bool,
    ) -> Result<(User, SetPasswordResult), AppError> {
        let display_name = User::validate_display_name(display_name)
            .map_err(|e| AppError::validation(format!("{e}")))?;
        let email = match email {
            Some(e) => Some(normalize_email(&e)?),
            None => None,
        };
        let username = match username {
            Some(u) => Some(
                User::validate_username(u).map_err(|e| AppError::validation(format!("{e}")))?,
            ),
            None => None,
        };
        if email.is_none() && username.is_none() {
            return Err(AppError::validation(
                "at least one of email or username is required",
            ));
        }
        let user = User {
            id: UserId::new(),
            display_name,
            email,
            username,
            status: UserStatus::Active,
            created_at: Utc::now(),
        };
        let user = self.users.create_user(user).await?;
        let pw = self
            .set_password(user.id, password, must_change)
            .await?;
        Ok((user, pw))
    }

    /// Rotate refresh: insert the new jti first, then revoke the old one.
    pub async fn refresh(&self, req: RefreshRequest) -> Result<TokenPair, AppError> {
        let claims = self.jwt.verify_refresh(&req.refresh_token)?;
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
        if !user.status.can_login() {
            return Err(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "account is not active",
            ));
        }
        let issued = self.jwt.issue_pair(user.id, user.email.clone())?;
        self.refresh
            .insert(issued.refresh_jti, issued.user_id, issued.refresh_exp)
            .await?;
        let was_active = self.refresh.revoke(claims.jti).await?;
        if !was_active {
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

    pub async fn must_change_password(&self, user_id: UserId) -> Result<bool, AppError> {
        Ok(self
            .credentials
            .get(user_id)
            .await?
            .map(|c| c.must_change_password)
            .unwrap_or(false))
    }
}

/// Convenience factory for fully in-memory auth (tests + local dev without PG).
pub type MemoryAuthService = AuthService<
    crate::store::InMemoryUserStore,
    crate::store::InMemoryOtpStore,
    crate::store::InMemoryRefreshStore,
    crate::credentials::InMemoryCredentialStore,
>;

impl MemoryAuthService {
    /// In-memory auth for local dev + integration tests (fixed OTP `123456`).
    pub fn memory_dev() -> Self {
        use crate::jwt::JwtConfig;
        use crate::otp::OtpConfig;
        use crate::store::{InMemoryOtpStore, InMemoryRefreshStore, InMemoryUserStore};

        let jwt = JwtService::new(JwtConfig::from_env());
        let otp = OtpService::new(InMemoryOtpStore::default(), OtpConfig::dev());
        Self::with_notifier(
            InMemoryUserStore::default(),
            otp,
            InMemoryRefreshStore::default(),
            crate::credentials::InMemoryCredentialStore::default(),
            jwt,
            Arc::new(crate::notifier::NoopOtpNotifier),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::InMemoryCredentialStore;
    use crate::jwt::JwtConfig;
    use crate::notifier::OtpNotifier;
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
            InMemoryCredentialStore::default(),
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
    async fn password_provision_and_login() {
        let svc = service();
        let (user, set) = svc
            .provision_user(
                "Host One".into(),
                Some("host1@example.com".into()),
                Some("host1".into()),
                Some("secret-pass-1"),
                false,
            )
            .await
            .unwrap();
        assert!(set.temporary_password.is_none());
        assert!(!set.must_change_password);
        let session = svc
            .password_login(PasswordLoginRequest {
                identifier: "host1".into(),
                password: "secret-pass-1".into(),
            })
            .await
            .unwrap();
        assert_eq!(session.user.id, user.id);
        assert!(!session.must_change_password);

        let err = svc
            .password_login(PasswordLoginRequest {
                identifier: "host1".into(),
                password: "wrong-pass".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidCredentials);
    }

    #[tokio::test]
    async fn password_lockout_after_max_attempts() {
        let svc = service().with_password_policy(PasswordPolicy {
            min_len: 8,
            max_attempts: 3,
            lock_secs: 60,
        });
        let _ = svc
            .provision_user(
                "Lock Me".into(),
                Some("lock@example.com".into()),
                Some("lockme".into()),
                Some("goodpassword"),
                false,
            )
            .await
            .unwrap();
        for _ in 0..3 {
            let _ = svc
                .password_login(PasswordLoginRequest {
                    identifier: "lockme".into(),
                    password: "badpassword".into(),
                })
                .await;
        }
        let err = svc
            .password_login(PasswordLoginRequest {
                identifier: "lockme".into(),
                password: "goodpassword".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthAccountLocked);
    }

    #[tokio::test]
    async fn change_password_revokes_sessions() {
        let svc = service();
        let (user, _) = svc
            .provision_user(
                "Changer".into(),
                Some("ch@example.com".into()),
                Some("changer".into()),
                Some("oldpassword1"),
                false,
            )
            .await
            .unwrap();
        let session = svc
            .password_login(PasswordLoginRequest {
                identifier: "changer".into(),
                password: "oldpassword1".into(),
            })
            .await
            .unwrap();
        svc.change_password(
            user.id,
            ChangePasswordRequest {
                current_password: "oldpassword1".into(),
                new_password: "newpassword2".into(),
            },
        )
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
        let err = svc
            .verify_otp(OtpVerifyRequest {
                email: "bad@example.com".into(),
                code: "999999".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AuthInvalidOtp);
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
            InMemoryCredentialStore::default(),
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
