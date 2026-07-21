use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::service_info;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyResponse {
    pub ready: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MetaResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub api_version: &'static str,
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
    let info = service_info();
    Json(HealthResponse {
        status: "ok",
        service: info.name,
    })
}

/// Readiness probe (P0: always ready; later checks DB/NATS).
#[utoipa::path(
    get,
    path = "/ready",
    tag = "system",
    responses(
        (status = 200, description = "Service is ready", body = ReadyResponse)
    )
)]
pub async fn ready() -> Json<ReadyResponse> {
    Json(ReadyResponse { ready: true })
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
pub async fn meta() -> Json<MetaResponse> {
    let info = service_info();
    Json(MetaResponse {
        name: info.name,
        version: info.version,
        api_version: "v1",
    })
}
