use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::service_info;
use crate::state::AppState;

/// Process-local counters for the minimal Prometheus scrape surface.
static HTTP_REQUESTS: AtomicU64 = AtomicU64::new(0);
static STARTED_UNIX: AtomicU64 = AtomicU64::new(0);

/// Call once at process start (or first scrape) so start time is stable.
pub fn mark_process_start() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = STARTED_UNIX.compare_exchange(0, now, Ordering::SeqCst, Ordering::SeqCst);
}

/// Increment on each observed request through metrics-aware handlers.
pub fn note_http_request() {
    HTTP_REQUESTS.fetch_add(1, Ordering::Relaxed);
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyResponse {
    pub ready: bool,
}

/// Process feature kill-switches exposed for clients (soft-hide experimental UI).
#[derive(Debug, Serialize, ToSchema)]
pub struct MetaFeatures {
    pub public_register: bool,
    pub real_pay: bool,
    /// P3 PK — default OFF when unset (plan 06).
    pub pk: bool,
    /// P3 co-host — default OFF when unset (plan 06).
    pub cohost: bool,
    pub client_events: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MetaResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub api_version: &'static str,
    /// Broadcast media backend (`srs` | `cloudflare_stream`).
    pub media_provider: &'static str,
    /// GA / experimental kill-switches (P1-safe defaults: pk/cohost false).
    pub features: MetaFeatures,
}

/// Liveness probe.
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Service is alive", body = HealthResponse)
    )
)]
pub async fn health() -> Json<HealthResponse> {
    note_http_request();
    let info = service_info();
    Json(HealthResponse {
        status: "ok",
        service: info.name,
    })
}

/// Readiness probe: always ready for memory mode; pings Postgres when enabled.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "system",
    responses(
        (status = 200, description = "Service is ready", body = ReadyResponse),
        (status = 503, description = "Dependency not ready", body = ReadyResponse)
    )
)]
pub async fn ready(State(state): State<Arc<AppState>>) -> (axum::http::StatusCode, Json<ReadyResponse>) {
    note_http_request();
    if let Some(pool) = state.db.as_ref() {
        match anylive_db::ping(pool).await {
            Ok(()) => (
                axum::http::StatusCode::OK,
                Json(ReadyResponse { ready: true }),
            ),
            Err(err) => {
                tracing::warn!(error = %err, "ready: postgres ping failed");
                (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    Json(ReadyResponse { ready: false }),
                )
            }
        }
    } else {
        (
            axum::http::StatusCode::OK,
            Json(ReadyResponse { ready: true }),
        )
    }
}

/// Public service metadata.
#[utoipa::path(
    get,
    path = "/api/v1/meta",
    tag = "system",
    responses(
        (status = 200, description = "API metadata", body = MetaResponse)
    )
)]
pub async fn meta(State(state): State<Arc<AppState>>) -> Json<MetaResponse> {
    note_http_request();
    let info = service_info();
    let f = &state.features;
    Json(MetaResponse {
        name: info.name,
        version: info.version,
        api_version: "v1",
        media_provider: anylive_media::media_provider_kind_from_env(),
        features: MetaFeatures {
            public_register: f.public_register,
            real_pay: f.real_pay,
            pk: f.pk,
            cohost: f.cohost,
            client_events: f.client_events,
        },
    })
}

/// Minimal Prometheus text exposition (P1 observability hook).
///
/// Full OTel/Prometheus pipeline is P2; this keeps scrapers and stage health
/// checks unblocked without extra crates. Includes process gauges for feature
/// flags and in-memory analytics retention.
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "system",
    responses(
        (status = 200, description = "Prometheus text metrics")
    )
)]
pub async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    mark_process_start();
    note_http_request();
    let reqs = HTTP_REQUESTS.load(Ordering::Relaxed);
    let start = STARTED_UNIX.load(Ordering::Relaxed);
    let events = state.analytics.count().await;
    let f = &state.features;
    let body = format!(
        "# HELP anylive_up 1 if the process is serving\n\
         # TYPE anylive_up gauge\n\
         anylive_up 1\n\
         # HELP anylive_http_requests_total Requests observed by the metrics helpers\n\
         # TYPE anylive_http_requests_total counter\n\
         anylive_http_requests_total {reqs}\n\
         # HELP anylive_process_start_time_seconds Unix start time\n\
         # TYPE anylive_process_start_time_seconds gauge\n\
         anylive_process_start_time_seconds {start}\n\
         # HELP anylive_analytics_events_buffered Client events retained in process\n\
         # TYPE anylive_analytics_events_buffered gauge\n\
         anylive_analytics_events_buffered {events}\n\
         # HELP anylive_feature_enabled Feature kill-switch (1=on)\n\
         # TYPE anylive_feature_enabled gauge\n\
         anylive_feature_enabled{{name=\"public_register\"}} {}\n\
         anylive_feature_enabled{{name=\"real_pay\"}} {}\n\
         anylive_feature_enabled{{name=\"pk\"}} {}\n\
         anylive_feature_enabled{{name=\"cohost\"}} {}\n\
         anylive_feature_enabled{{name=\"client_events\"}} {}\n",
        u8::from(f.public_register),
        u8::from(f.real_pay),
        u8::from(f.pk),
        u8::from(f.cohost),
        u8::from(f.client_events),
    );
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
