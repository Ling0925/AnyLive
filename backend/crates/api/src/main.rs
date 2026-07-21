//! AnyLive HTTP API binary.

use anylive_api::build_app;
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
    let app = build_app();

    tracing::info!(%addr, "anylive-api listening (auth: in-memory dev OTP=123456)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
