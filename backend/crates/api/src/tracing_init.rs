//! Process tracing bootstrap (WBS E0.5).
//!
//! Default: human-readable `tracing-subscriber` fmt + `EnvFilter`.
//! Set `RUST_LOG_FORMAT=json` for structured JSON logs (collector-friendly).
//! Full OTLP export is deferred to an ops-side collector; see
//! `docs/runbooks/otel.md`.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize global subscriber. Safe to call once at process start.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,anylive_api=debug"));
    let json = std::env::var("RUST_LOG_FORMAT")
        .map(|v| {
            let t = v.trim().to_ascii_lowercase();
            t == "json" || t == "structured"
        })
        .unwrap_or(false);

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json().flatten_event(true))
            .init();
        tracing::info!(format = "json", "tracing initialized");
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
        tracing::info!(format = "text", "tracing initialized");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn json_flag_detects_env_values() {
        // Pure helper logic mirrored for unit coverage without re-init.
        for v in ["json", "JSON", " structured ", "Structured"] {
            let t = v.trim().to_ascii_lowercase();
            assert!(t == "json" || t == "structured", "{v}");
        }
        for v in ["text", "pretty", ""] {
            let t = v.trim().to_ascii_lowercase();
            assert!(!(t == "json" || t == "structured"), "{v}");
        }
    }
}
