//! P1 compliance stubs: account export/delete + legal links.

use std::collections::HashSet;
use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::UserId;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use tokio::sync::Mutex;
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::routes::auth::UserDto;
use crate::state::AppState;

/// In-memory set of soft-deleted user IDs (P1 stub).
#[derive(Clone, Default)]
pub struct DeletedUsers {
    inner: Arc<Mutex<HashSet<UserId>>>,
}

impl DeletedUsers {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn mark_deleted(&self, user_id: UserId) {
        self.inner.lock().await.insert(user_id);
    }

    pub async fn is_deleted(&self, user_id: UserId) -> bool {
        self.inner.lock().await.contains(&user_id)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountExportDto {
    pub user: UserDto,
    pub rooms_owned_count: u64,
    pub note: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LegalDocDto {
    pub url: String,
    pub version: String,
    pub title: String,
}

/// Export current account data (P1 stub).
#[utoipa::path(
    get,
    path = "/api/v1/me/export",
    tag = "compliance",
    security(("bearerAuth" = [])),
    responses((status = 200, body = AccountExportDto))
)]
pub async fn export_me(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<AccountExportDto>, ApiError> {
    if state.deleted_users.is_deleted(user.user_id).await {
        return Err(ApiError(AppError::unauthorized("account deleted")));
    }
    let u = state.auth.me(user.user_id).await.map_err(ApiError::from)?;
    Ok(Json(AccountExportDto {
        user: u.into(),
        rooms_owned_count: 0,
        note: "P1 export stub".into(),
    }))
}

/// Soft-delete account: revoke refresh tokens and mark deleted (P1 stub).
#[utoipa::path(
    delete,
    path = "/api/v1/me",
    tag = "compliance",
    security(("bearerAuth" = [])),
    responses((status = 204, description = "Account deleted"))
)]
pub async fn delete_me(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<StatusCode, ApiError> {
    if state.deleted_users.is_deleted(user.user_id).await {
        return Err(ApiError(AppError::unauthorized("account deleted")));
    }
    // Revoke all refresh tokens (logout-all).
    state
        .auth
        .logout(user.user_id, None)
        .await
        .map_err(ApiError::from)?;
    state.deleted_users.mark_deleted(user.user_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Privacy policy link (public).
#[utoipa::path(
    get,
    path = "/api/v1/legal/privacy",
    tag = "compliance",
    responses((status = 200, body = LegalDocDto))
)]
pub async fn legal_privacy() -> Json<LegalDocDto> {
    Json(LegalDocDto {
        url: "https://anylive.example/privacy".into(),
        version: "1.0".into(),
        title: "Privacy Policy".into(),
    })
}

/// Terms of service link (public).
#[utoipa::path(
    get,
    path = "/api/v1/legal/terms",
    tag = "compliance",
    responses((status = 200, body = LegalDocDto))
)]
pub async fn legal_terms() -> Json<LegalDocDto> {
    Json(LegalDocDto {
        url: "https://anylive.example/terms".into(),
        version: "1.0".into(),
        title: "Terms of Service".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{build_app_with_state, AppState};

    async fn body_json(res: axum::response::Response) -> serde_json::Value {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        if bytes.is_empty() {
            return serde_json::Value::Null;
        }
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn login(app: &axum::Router, email: &str) -> String {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/send")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"email":"{email}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/otp/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","code":"123456"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        json["access_token"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn legal_privacy_and_terms_public() {
        let app = build_app_with_state(AppState::dev());
        for (path, title, url) in [
            (
                "/api/v1/legal/privacy",
                "Privacy Policy",
                "https://anylive.example/privacy",
            ),
            (
                "/api/v1/legal/terms",
                "Terms of Service",
                "https://anylive.example/terms",
            ),
        ] {
            let res = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            let json = body_json(res).await;
            assert_eq!(json["title"], title);
            assert_eq!(json["url"], url);
            assert_eq!(json["version"], "1.0");
        }
    }

    #[tokio::test]
    async fn export_me_returns_stub() {
        let app = build_app_with_state(AppState::dev());
        let access = login(&app, "export@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me/export")
                    .header("authorization", format!("Bearer {access}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let json = body_json(res).await;
        assert_eq!(json["note"], "P1 export stub");
        assert_eq!(json["rooms_owned_count"], 0);
        assert_eq!(json["user"]["email"], "export@example.com");
    }

    #[tokio::test]
    async fn delete_me_revokes_and_blocks_subsequent_access() {
        let app = build_app_with_state(AppState::dev());
        let access = login(&app, "delete-me@example.com").await;

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {access}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        // Access token still valid JWT-wise, but account is marked deleted → 401.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me")
                    .header("authorization", format!("Bearer {access}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/me/export")
                    .header("authorization", format!("Bearer {access}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn deleted_users_set_works() {
        let set = DeletedUsers::new();
        let id = UserId::new();
        assert!(!set.is_deleted(id).await);
        set.mark_deleted(id).await;
        assert!(set.is_deleted(id).await);
    }
}
