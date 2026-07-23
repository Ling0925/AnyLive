//! Client analytics event batch ingest (P4 scaffold).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::analytics::ClientEventInput;
use crate::auth_user::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ClientEventDto {
    pub name: String,
    #[serde(default)]
    pub occurred_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub props: Option<serde_json::Value>,
    #[serde(default)]
    pub client_event_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ClientEventBatchBody {
    pub events: Vec<ClientEventDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClientEventIngestResponse {
    pub accepted: i64,
    pub dropped: i64,
}

/// Accept a batch of client analytics events (auth required).
#[utoipa::path(
    post,
    path = "/api/v1/events",
    tag = "analytics",
    security(("bearerAuth" = [])),
    responses((status = 202, body = ClientEventIngestResponse))
)]
pub async fn ingest_events(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<ClientEventBatchBody>,
) -> Result<(StatusCode, Json<ClientEventIngestResponse>), ApiError> {
    state.features.require_client_events().map_err(ApiError::from)?;
    if body.events.is_empty() {
        return Ok((
            StatusCode::ACCEPTED,
            Json(ClientEventIngestResponse {
                accepted: 0,
                dropped: 0,
            }),
        ));
    }
    if body.events.len() > crate::analytics::MAX_BATCH_SIZE {
        return Err(ApiError(anylive_common::AppError::validation(format!(
            "max {} events per batch",
            crate::analytics::MAX_BATCH_SIZE
        ))));
    }
    let inputs: Vec<ClientEventInput> = body
        .events
        .into_iter()
        .map(|e| ClientEventInput {
            name: e.name,
            occurred_at: e.occurred_at,
            props: e.props,
            client_event_id: e.client_event_id,
        })
        .collect();
    let (accepted, dropped) = state.analytics.ingest(user.user_id, inputs).await;
    Ok((
        StatusCode::ACCEPTED,
        Json(ClientEventIngestResponse {
            accepted: accepted as i64,
            dropped: dropped as i64,
        }),
    ))
}
