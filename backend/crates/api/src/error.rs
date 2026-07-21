use anylive_common::{AppError, ErrorCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Axum-compatible API error.
#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.0.code.status();
        let body = self.0.into_body();
        (status, Json(body)).into_response()
    }
}

/// Helper for mapping unexpected errors.
#[allow(dead_code)]
pub fn internal(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %err, "internal error");
    ApiError(AppError::new(ErrorCode::Internal, "internal server error"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn maps_status_from_code() {
        let err = ApiError(AppError::validation("x"));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
