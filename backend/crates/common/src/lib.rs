//! Shared error codes and API error envelope for AnyLive.

use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable machine-readable error codes (see contracts/errors/codes.yaml).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Internal,
    NotFound,
    Unauthorized,
    Forbidden,
    Validation,
    Conflict,
    RateLimited,
    AuthInvalidOtp,
    AuthInvalidCredentials,
    AuthTokenRevoked,
    AuthAccountLocked,
    RoomNotLive,
    MediaProviderError,
    GiftInsufficientBalance,
    GiftIdempotentReplay,
    WalletConflict,
    ForbiddenPolicy,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL",
            Self::NotFound => "NOT_FOUND",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Validation => "VALIDATION",
            Self::Conflict => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::AuthInvalidOtp => "AUTH_INVALID_OTP",
            Self::AuthInvalidCredentials => "AUTH_INVALID_CREDENTIALS",
            Self::AuthTokenRevoked => "AUTH_TOKEN_REVOKED",
            Self::AuthAccountLocked => "AUTH_ACCOUNT_LOCKED",
            Self::RoomNotLive => "ROOM_NOT_LIVE",
            Self::MediaProviderError => "MEDIA_PROVIDER_ERROR",
            Self::GiftInsufficientBalance => "GIFT_INSUFFICIENT_BALANCE",
            Self::GiftIdempotentReplay => "GIFT_IDEMPOTENT_REPLAY",
            Self::WalletConflict => "WALLET_CONFLICT",
            Self::ForbiddenPolicy => "FORBIDDEN_POLICY",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            Self::Internal | Self::MediaProviderError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized
            | Self::AuthInvalidOtp
            | Self::AuthInvalidCredentials
            | Self::AuthTokenRevoked => StatusCode::UNAUTHORIZED,
            Self::AuthAccountLocked => StatusCode::TOO_MANY_REQUESTS,
            Self::Forbidden | Self::ForbiddenPolicy => StatusCode::FORBIDDEN,
            Self::Validation => StatusCode::BAD_REQUEST,
            Self::Conflict | Self::RoomNotLive | Self::WalletConflict => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::GiftInsufficientBalance => StatusCode::PAYMENT_REQUIRED,
            Self::GiftIdempotentReplay => StatusCode::OK,
        }
    }

    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::Internal | Self::MediaProviderError | Self::RateLimited | Self::WalletConflict
        )
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// JSON error body returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub retryable: bool,
}

impl ApiErrorBody {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
            request_id: None,
            retryable: code.retryable(),
        }
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

/// Application error carrying a stable code.
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Validation, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    pub fn into_body(self) -> ApiErrorBody {
        ApiErrorBody::new(self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_roundtrip_strings() {
        assert_eq!(ErrorCode::GiftInsufficientBalance.as_str(), "GIFT_INSUFFICIENT_BALANCE");
        assert_eq!(
            ErrorCode::GiftInsufficientBalance.status(),
            StatusCode::PAYMENT_REQUIRED
        );
        assert!(!ErrorCode::GiftInsufficientBalance.retryable());
        assert!(ErrorCode::RateLimited.retryable());
    }

    #[test]
    fn api_error_body_serializes_code() {
        let body = ApiErrorBody::new(ErrorCode::Validation, "bad input").with_request_id("r1");
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["code"], "VALIDATION");
        assert_eq!(json["request_id"], "r1");
        assert_eq!(json["retryable"], false);
    }

    #[test]
    fn app_error_helpers() {
        let e = AppError::not_found("room");
        assert_eq!(e.code, ErrorCode::NotFound);
        let body = e.into_body();
        assert_eq!(body.code, "NOT_FOUND");
    }
}
