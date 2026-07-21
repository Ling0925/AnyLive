//! Bearer JWT authentication extractor.

use std::sync::Arc;

use anylive_auth::AccessClaims;
use anylive_common::{AppError, ErrorCode};
use anylive_domain::UserId;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::ApiError;
use crate::state::AppState;

/// Authenticated request context.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: UserId,
    pub email: Option<String>,
    pub claims: AccessClaims,
}

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError(AppError::unauthorized("missing Authorization header")))?;

        let token = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
            .ok_or_else(|| ApiError(AppError::unauthorized("expected Bearer token")))?
            .trim();

        if token.is_empty() {
            return Err(ApiError(AppError::unauthorized("empty bearer token")));
        }

        let claims = state.auth.jwt().verify_access(token).map_err(|e| {
            if e.code == ErrorCode::AuthTokenRevoked {
                ApiError(e)
            } else {
                ApiError(AppError::unauthorized(e.message))
            }
        })?;

        let user_id = UserId(claims.sub);
        // Enforce bans on every authenticated request (chat/gift/room/admin).
        if state.moderation.is_banned(user_id).await {
            return Err(ApiError(AppError::new(
                ErrorCode::ForbiddenPolicy,
                "user is banned",
            )));
        }

        Ok(AuthUser {
            user_id,
            email: claims.email.clone(),
            claims,
        })
    }
}
