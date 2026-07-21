//! AnyLive HTTP API binary.

use anylive_api::{build_app_with_state, AppState};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,anylive_api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let bind = std::env::var("API_BIND").unwrap_or_else(|_| "0.0.0.0:8088".to_string());
    let addr: SocketAddr = bind.parse()?;

    // Fail closed on production misconfig (default JWT / fixed OTP / default Centrifugo secret).
    // Optionally connects Postgres when USE_POSTGRES=1 and DATABASE_URL are set.
    let state = AppState::from_env()
        .await
        .map_err(|e| anyhow::anyhow!("startup guard failed: {e}"))?;
    let backend = if state.db.is_some() {
        "postgres"
    } else {
        "memory"
    };
    let app = build_app_with_state(state);

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into());
    tracing::info!(%addr, %app_env, backend, "anylive-api listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
