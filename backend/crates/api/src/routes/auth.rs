//! Auth HTTP handlers.

use std::sync::Arc;

use anylive_auth::{
    LogoutRequest, OtpSendRequest, OtpVerifyRequest, RefreshRequest, RefreshStore, TokenPair,
};
use anylive_domain::User;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct OtpSendBody {
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OtpVerifyBody {
    pub email: String,
    pub code: String,
    /// Soft-launch invite code (required when INVITE_ONLY=1 and email not allowlisted).
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OauthExchangeBody {
    /// google | apple
    pub provider: String,
    /// Provider ID token. Local/dogfood: `stub:<email>` when OAUTH_STUB/local.
    pub id_token: String,
    #[serde(default)]
    pub invite_code: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshBody {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutBody {
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSessionResponse {
    pub user: UserDto,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDto {
    pub id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub created_at: String,
    /// True when the user has confirmed age eligibility.
    pub age_confirmed: bool,
    /// True when the user has accepted the privacy policy.
    pub privacy_accepted: bool,
    /// Public avatar object URL when set (WBS E2.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Optional ISO 3166-1 alpha-2 region code (WBS E2.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

impl UserDto {
    pub fn from_user(
        u: User,
        age_confirmed: bool,
        privacy_accepted: bool,
        avatar_url: Option<String>,
        region: Option<String>,
    ) -> Self {
        Self {
            id: u.id.0.to_string(),
            display_name: u.display_name,
            email: u.email,
            created_at: u.created_at.to_rfc3339(),
            age_confirmed,
            privacy_accepted,
            avatar_url,
            region,
        }
    }
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self::from_user(u, false, false, None, None)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchMeBody {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub age_confirmed: Option<bool>,
    #[serde(default)]
    pub privacy_accepted: Option<bool>,
    /// ISO region code; empty string clears.
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenPairDto {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

impl From<TokenPair> for TokenPairDto {
    fn from(t: TokenPair) -> Self {
        Self {
            access_token: t.access_token,
            refresh_token: t.refresh_token,
            expires_in: t.expires_in,
        }
    }
}

/// Best-effort client IP for rate limiting (proxy-aware).
fn client_ip(headers: &HeaderMap, connect: Option<&std::net::SocketAddr>) -> String {
    if let Some(xff) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = xff.split(',').next() {
            let ip = first.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }
    if let Some(real) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let ip = real.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    connect
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Send email OTP (dev: always 123456 when ALLOW_DEV_OTP / local fixed mode).
#[utoipa::path(
    post,
    path = "/api/v1/auth/otp/send",
    tag = "auth",
    request_body = OtpSendBody,
    responses((status = 204, description = "OTP accepted"))
)]
pub async fn otp_send(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<OtpSendBody>,
) -> Result<StatusCode, ApiError> {
    let ip = client_ip(&headers, None);
    state
        .otp_ip_limiter
        .check(&format!("otp-send:{ip}"))
        .await
        .map_err(ApiError::from)?;
    // Also throttle per email (in addition to OTP service resend cooldown).
    let email_key = body.email.trim().to_lowercase();
    if !email_key.is_empty() {
        state
            .otp_ip_limiter
            .check(&format!("otp-send-email:{email_key}"))
            .await
            .map_err(ApiError::from)?;
    }
    state
        .auth
        .send_otp(OtpSendRequest { email: body.email })
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Verify OTP and issue tokens.
#[utoipa::path(
    post,
    path = "/api/v1/auth/otp/verify",
    tag = "auth",
    request_body = OtpVerifyBody,
    responses((status = 200, description = "Session", body = AuthSessionResponse))
)]
pub async fn otp_verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<OtpVerifyBody>,
) -> Result<Json<AuthSessionResponse>, ApiError> {
    let ip = client_ip(&headers, None);
    state
        .otp_ip_limiter
        .check(&format!("otp-verify:{ip}"))
        .await
        .map_err(ApiError::from)?;

    if !state.features.public_register && !state.invite.is_enabled() {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::ForbiddenPolicy,
            "public registration disabled (FEATURE_PUBLIC_REGISTER=0); enable INVITE_ONLY allowlist/codes or re-enable the flag",
        )));
    }

    state
        .invite
        .check(&body.email, body.invite_code.as_deref())
        .map_err(ApiError::from)?;

    // Gate deleted + banned **before** issuing tokens / inserting refresh rows.
    let session = state
        .auth
        .verify_otp_gated(
            OtpVerifyRequest {
                email: body.email,
                code: body.code,
            },
            |user| {
                let state = state.clone();
                async move {
                    if state.deleted_users.is_deleted(user.id).await {
                        return Err(anylive_common::AppError::unauthorized("account deleted"));
                    }
                    match state.moderation.try_is_banned(user.id).await {
                        Ok(true) => Err(anylive_common::AppError::new(
                            anylive_common::ErrorCode::ForbiddenPolicy,
                            "user is banned",
                        )),
                        Ok(false) => Ok(user),
                        Err(e) => Err(e),
                    }
                }
            },
        )
        .await
        .map_err(ApiError::from)?;

    let extras = state.profile_extras.get(session.user.id).await;
    Ok(Json(AuthSessionResponse {
        user: UserDto::from_user(
            session.user,
            extras.age_confirmed(),
            extras.privacy_accepted(),
            extras.avatar_url.clone(),
            extras.region.clone(),
        ),
        access_token: session.tokens.access_token,
        refresh_token: session.tokens.refresh_token,
        expires_in: session.tokens.expires_in,
    }))
}

/// OAuth exchange scaffold (WBS E2.1).
///
/// Local/dogfood: `id_token = "stub:<email>"` when stub mode is on.
/// Production: stub disabled; real JWKS verification still requires vendor config.
#[utoipa::path(
    post,
    path = "/api/v1/auth/oauth/exchange",
    tag = "auth",
    request_body = OauthExchangeBody,
    responses((status = 200, description = "Session", body = AuthSessionResponse))
)]
pub async fn oauth_exchange(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<OauthExchangeBody>,
) -> Result<Json<AuthSessionResponse>, ApiError> {
    let ip = client_ip(&headers, None);
    state
        .otp_ip_limiter
        .check(&format!("oauth-exchange:{ip}"))
        .await
        .map_err(ApiError::from)?;

    if !state.features.public_register && !state.invite.is_enabled() {
        return Err(ApiError(anylive_common::AppError::new(
            anylive_common::ErrorCode::ForbiddenPolicy,
            "public registration disabled (FEATURE_PUBLIC_REGISTER=0)",
        )));
    }

    let email = crate::oauth::resolve_oauth_email(
        &state.oauth,
        &body.provider,
        &body.id_token,
    )
    .map_err(ApiError::from)?;

    state
        .invite
        .check(&email, body.invite_code.as_deref())
        .map_err(ApiError::from)?;

    let session = state
        .auth
        .login_by_email_gated(&email, |user| {
            let state = state.clone();
            async move {
                if state.deleted_users.is_deleted(user.id).await {
                    return Err(anylive_common::AppError::unauthorized("account deleted"));
                }
                match state.moderation.try_is_banned(user.id).await {
                    Ok(true) => Err(anylive_common::AppError::new(
                        anylive_common::ErrorCode::ForbiddenPolicy,
                        "user is banned",
                    )),
                    Ok(false) => Ok(user),
                    Err(e) => Err(e),
                }
            }
        })
        .await
        .map_err(ApiError::from)?;

    let extras = state.profile_extras.get(session.user.id).await;
    Ok(Json(AuthSessionResponse {
        user: UserDto::from_user(
            session.user,
            extras.age_confirmed(),
            extras.privacy_accepted(),
            extras.avatar_url.clone(),
            extras.region.clone(),
        ),
        access_token: session.tokens.access_token,
        refresh_token: session.tokens.refresh_token,
        expires_in: session.tokens.expires_in,
    }))
}

/// Rotate refresh token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/token/refresh",
    tag = "auth",
    request_body = RefreshBody,
    responses((status = 200, body = TokenPairDto))
)]
pub async fn token_refresh(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshBody>,
) -> Result<Json<TokenPairDto>, ApiError> {
    // Enforce ban/delete before rotating — refresh does not go through AuthUser.
    let claims = state
        .auth
        .jwt()
        .verify_refresh(&body.refresh_token)
        .map_err(ApiError::from)?;
    let user_id = anylive_domain::UserId(claims.sub);
    if state.deleted_users.is_deleted(user_id).await {
        return Err(ApiError(anylive_common::AppError::unauthorized(
            "account deleted",
        )));
    }
    match state.moderation.try_is_banned(user_id).await {
        Ok(true) => {
            return Err(ApiError(anylive_common::AppError::new(
                anylive_common::ErrorCode::ForbiddenPolicy,
                "user is banned",
            )));
        }
        Ok(false) => {}
        Err(e) => return Err(ApiError(e)),
    }
    let pair = state
        .auth
        .refresh(RefreshRequest {
            refresh_token: body.refresh_token,
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(pair.into()))
}

/// Logout and revoke refresh token(s).
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    security(("bearerAuth" = [])),
    request_body = LogoutBody,
    responses((status = 204, description = "Logged out"))
)]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    body: Option<Json<LogoutBody>>,
) -> Result<StatusCode, ApiError> {
    let refresh = body.and_then(|Json(b)| b.refresh_token);
    let _ = LogoutRequest {
        refresh_token: refresh.clone(),
    };
    state
        .auth
        .logout(user.user_id, refresh.as_deref())
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionDto {
    pub jti: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionListResponse {
    pub items: Vec<SessionDto>,
}

/// List active refresh sessions for the current user (WBS E2.4, jti-only, no device meta).
#[utoipa::path(
    get,
    path = "/api/v1/me/sessions",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses((status = 200, body = SessionListResponse))
)]
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<SessionListResponse>, ApiError> {
    let items = state
        .auth
        .refresh_store()
        .list_for_user(user.user_id)
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .map(|s| SessionDto {
            jti: s.jti.to_string(),
            expires_at: chrono::DateTime::<chrono::Utc>::from_timestamp(s.exp, 0)
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| s.exp.to_string()),
        })
        .collect();
    Ok(Json(SessionListResponse { items }))
}

/// Revoke all refresh sessions for the current user (logout-all).
#[utoipa::path(
    delete,
    path = "/api/v1/me/sessions",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses((status = 200, description = "Revoked count", body = LogoutAllResponse))
)]
pub async fn logout_all_sessions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<LogoutAllResponse>, ApiError> {
    let revoked = state
        .auth
        .refresh_store()
        .revoke_all_for_user(user.user_id)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(LogoutAllResponse {
        revoked: revoked as u64,
    }))
}

/// Revoke a single refresh session by jti (must belong to the caller).
#[utoipa::path(
    delete,
    path = "/api/v1/me/sessions/{jti}",
    tag = "auth",
    security(("bearerAuth" = [])),
    params(("jti" = String, Path, description = "Refresh token jti (uuid)")),
    responses(
        (status = 204, description = "Session revoked"),
        (status = 404, description = "Session not found or not owned")
    )
)]
pub async fn revoke_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(jti): Path<String>,
) -> Result<StatusCode, ApiError> {
    let jti = Uuid::parse_str(jti.trim()).map_err(|_| {
        ApiError::from(anylive_common::AppError::validation("invalid session jti"))
    })?;
    let revoked = state
        .auth
        .refresh_store()
        .revoke_for_user(jti, user.user_id)
        .await
        .map_err(ApiError::from)?;
    if !revoked {
        return Err(ApiError::from(anylive_common::AppError::not_found(
            "session not found",
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutAllResponse {
    pub revoked: u64,
}

/// Current user profile.
#[utoipa::path(
    get,
    path = "/api/v1/me",
    tag = "users",
    security(("bearerAuth" = [])),
    responses((status = 200, body = UserDto))
)]
pub async fn me(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<UserDto>, ApiError> {
    let u = state.auth.me(user.user_id).await.map_err(ApiError::from)?;
    let extras = state.profile_extras.get(user.user_id).await;
    Ok(Json(UserDto::from_user(
        u,
        extras.age_confirmed(),
        extras.privacy_accepted(),
        extras.avatar_url.clone(),
        extras.region.clone(),
    )))
}

/// Normalize optional region code: empty → clear; else uppercase 2-letter-ish.
fn normalize_region(raw: Option<String>) -> Result<Option<Option<String>>, ApiError> {
    let Some(s) = raw else {
        return Ok(None);
    };
    let t = s.trim();
    if t.is_empty() {
        return Ok(Some(None));
    }
    let upper = t.to_ascii_uppercase();
    if upper.len() < 2 || upper.len() > 8 || !upper.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "region must be 2–8 alphanumeric characters (e.g. US, SG)",
        )));
    }
    Ok(Some(Some(upper)))
}

/// Patch current user profile (display name, age/privacy declarations, region).
#[utoipa::path(
    patch,
    path = "/api/v1/me",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = PatchMeBody,
    responses((status = 200, body = UserDto))
)]
pub async fn patch_me(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<PatchMeBody>,
) -> Result<Json<UserDto>, ApiError> {
    if body.display_name.is_none()
        && body.age_confirmed.is_none()
        && body.privacy_accepted.is_none()
        && body.region.is_none()
    {
        return Err(ApiError::from(anylive_common::AppError::validation(
            "at least one field required",
        )));
    }

    let u = if let Some(name) = body.display_name {
        state
            .auth
            .update_display_name(user.user_id, name)
            .await
            .map_err(ApiError::from)?
    } else {
        state.auth.me(user.user_id).await.map_err(ApiError::from)?
    };

    let region = normalize_region(body.region)?;
    let extras = state
        .profile_extras
        .patch(
            user.user_id,
            body.age_confirmed,
            body.privacy_accepted,
            region,
        )
        .await;

    Ok(Json(UserDto::from_user(
        u,
        extras.age_confirmed(),
        extras.privacy_accepted(),
        extras.avatar_url.clone(),
        extras.region.clone(),
    )))
}
