//! Auth HTTP handlers.

use std::sync::Arc;

use anylive_auth::{
    LogoutRequest, OtpSendRequest, OtpVerifyRequest, RefreshRequest, TokenPair,
};
use anylive_domain::User;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id.0.to_string(),
            display_name: u.display_name,
            email: u.email,
            created_at: u.created_at.to_rfc3339(),
        }
    }
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

/// Send email OTP (dev: always 123456).
#[utoipa::path(
    post,
    path = "/api/v1/auth/otp/send",
    tag = "auth",
    request_body = OtpSendBody,
    responses((status = 204, description = "OTP accepted"))
)]
pub async fn otp_send(
    State(state): State<Arc<AppState>>,
    Json(body): Json<OtpSendBody>,
) -> Result<StatusCode, ApiError> {
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
    Json(body): Json<OtpVerifyBody>,
) -> Result<Json<AuthSessionResponse>, ApiError> {
    let session = state
        .auth
        .verify_otp(OtpVerifyRequest {
            email: body.email,
            code: body.code,
        })
        .await
        .map_err(ApiError::from)?;
    Ok(Json(AuthSessionResponse {
        user: session.user.into(),
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
    Ok(Json(u.into()))
}
