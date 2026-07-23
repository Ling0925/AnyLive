//! Compliance: account export/delete + legal links.

use std::sync::Arc;

use anylive_common::AppError;
use anylive_domain::RoomStatus;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::routes::auth::UserDto;
use crate::state::AppState;

/// Cap ledger / room list size in a single export response.
const EXPORT_LEDGER_LIMIT: usize = 200;
const EXPORT_ROOMS_LIMIT: usize = 100;
const EXPORT_FOLLOWING_LIMIT: usize = 500;

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountExportDto {
    pub schema_version: String,
    pub exported_at: String,
    pub user: UserDto,
    pub profile: ExportProfileDto,
    pub rooms_owned: Vec<ExportRoomDto>,
    pub rooms_owned_count: u64,
    pub wallet: ExportWalletDto,
    pub social: ExportSocialDto,
    pub note: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportProfileDto {
    pub age_confirmed: bool,
    pub privacy_accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_confirmed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_accepted_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportRoomDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportWalletDto {
    pub balance: i64,
    pub ledger: Vec<ExportLedgerDto>,
    pub ledger_truncated: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportLedgerDto {
    pub id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub entry_type: String,
    pub reference: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportSocialDto {
    pub following_ids: Vec<String>,
    pub following_truncated: bool,
}

/// Export current account data (GDPR-oriented self-service dump).
///
/// Includes profile flags, owned rooms, wallet balance + recent ledger,
/// and following list. Chat message bodies and stream keys are omitted
/// (content retention / security).
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
    let extras = state.profile_extras.get(user.user_id).await;
    let age_confirmed = extras.age_confirmed();
    let privacy_accepted = extras.privacy_accepted();

    // Owned rooms: scan list (memory/PG list is P1-sized; filter by owner).
    let all_rooms = state.rooms.list(None).await;
    let mut owned: Vec<_> = all_rooms
        .into_iter()
        .filter(|r| r.owner_id == user.user_id)
        .collect();
    owned.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let rooms_owned_count = owned.len() as u64;
    let rooms_truncated = owned.len() > EXPORT_ROOMS_LIMIT;
    owned.truncate(EXPORT_ROOMS_LIMIT);
    let rooms_owned: Vec<ExportRoomDto> = owned
        .into_iter()
        .map(|r| ExportRoomDto {
            id: r.id.0.to_string(),
            title: r.title,
            status: match r.status {
                RoomStatus::Idle => "idle".into(),
                RoomStatus::Live => "live".into(),
                RoomStatus::Closed => "closed".into(),
            },
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        })
        .collect();

    let balance = state.wallet.balance(user.user_id).await;
    let mut ledger = state.wallet.ledger_for(user.user_id).await;
    // Newest first for export readability.
    ledger.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let ledger_truncated = ledger.len() > EXPORT_LEDGER_LIMIT;
    ledger.truncate(EXPORT_LEDGER_LIMIT);
    let ledger_dto: Vec<ExportLedgerDto> = ledger
        .into_iter()
        .map(|e| ExportLedgerDto {
            id: e.id.to_string(),
            amount: e.amount,
            balance_after: e.balance_after,
            entry_type: match e.entry_type {
                anylive_wallet::LedgerType::Topup => "topup".into(),
                anylive_wallet::LedgerType::GiftDebit => "gift_debit".into(),
                anylive_wallet::LedgerType::GiftCredit => "gift_credit".into(),
                anylive_wallet::LedgerType::Adjustment => "adjustment".into(),
            },
            reference: e.reference,
            created_at: e.created_at.to_rfc3339(),
        })
        .collect();

    let mut following = state.social.following_ids(user.user_id).await;
    let following_truncated = following.len() > EXPORT_FOLLOWING_LIMIT;
    following.truncate(EXPORT_FOLLOWING_LIMIT);
    let following_ids: Vec<String> = following.into_iter().map(|id| id.0.to_string()).collect();

    let mut notes = vec![
        "Account self-export".to_string(),
        "Omits chat message bodies, stream keys, refresh tokens, and OTP secrets".to_string(),
    ];
    if rooms_truncated {
        notes.push(format!("rooms list truncated to {EXPORT_ROOMS_LIMIT}"));
    }
    if ledger_truncated {
        notes.push(format!("ledger truncated to {EXPORT_LEDGER_LIMIT} newest entries"));
    }
    if following_truncated {
        notes.push(format!("following list truncated to {EXPORT_FOLLOWING_LIMIT}"));
    }

    Ok(Json(AccountExportDto {
        schema_version: "1.0".into(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        user: UserDto::from_user(u, age_confirmed, privacy_accepted, extras.avatar_url.clone(), extras.region.clone()),
        profile: ExportProfileDto {
            age_confirmed,
            privacy_accepted,
            age_confirmed_at: extras.age_confirmed_at.map(|t| t.to_rfc3339()),
            privacy_accepted_at: extras.privacy_accepted_at.map(|t| t.to_rfc3339()),
        },
        rooms_owned,
        rooms_owned_count,
        wallet: ExportWalletDto {
            balance,
            ledger: ledger_dto,
            ledger_truncated,
        },
        social: ExportSocialDto {
            following_ids,
            following_truncated,
        },
        note: notes.join("; "),
    }))
}

/// Soft-delete account: revoke refresh tokens and mark deleted.
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
        url: std::env::var("LEGAL_PRIVACY_URL")
            .unwrap_or_else(|_| "https://anylive.example/privacy".into()),
        version: std::env::var("LEGAL_PRIVACY_VERSION").unwrap_or_else(|_| "1.0".into()),
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
        url: std::env::var("LEGAL_TERMS_URL")
            .unwrap_or_else(|_| "https://anylive.example/terms".into()),
        version: std::env::var("LEGAL_TERMS_VERSION").unwrap_or_else(|_| "1.0".into()),
        title: "Terms of Service".into(),
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LegalDocDto {
    pub url: String,
    pub version: String,
    pub title: String,
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
        for (path, title) in [
            ("/api/v1/legal/privacy", "Privacy Policy"),
            ("/api/v1/legal/terms", "Terms of Service"),
        ] {
            let res = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            let json = body_json(res).await;
            assert_eq!(json["title"], title);
            assert!(json["url"].as_str().unwrap().starts_with("http"));
            assert_eq!(json["version"], "1.0");
        }
    }

    #[tokio::test]
    async fn export_me_includes_wallet_and_rooms() {
        let state = AppState::dev_ready().await;
        let app = build_app_with_state(state.clone());
        let access = login(&app, "export@example.com").await;

        // Create room + topup so export is non-empty.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("authorization", format!("Bearer {access}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Export Room"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/wallet/topups")
                    .header("authorization", format!("Bearer {access}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":42,"reference":"export-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

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
        assert_eq!(json["schema_version"], "1.0");
        assert_eq!(json["user"]["email"], "export@example.com");
        assert_eq!(json["rooms_owned_count"], 1);
        assert_eq!(json["rooms_owned"].as_array().unwrap().len(), 1);
        assert_eq!(json["wallet"]["balance"], 42);
        assert!(!json["wallet"]["ledger"].as_array().unwrap().is_empty());
        assert!(json["note"].as_str().unwrap().contains("Account self-export"));
        assert!(!json["note"].as_str().unwrap().contains("P1 export stub"));
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
        use anylive_db::MemoryDeletedUsers;
        use anylive_domain::UserId;

        let set = MemoryDeletedUsers::new();
        let id = UserId::new();
        assert!(!set.is_deleted(id).await);
        set.mark_deleted(id).await;
        assert!(set.is_deleted(id).await);
    }
}
