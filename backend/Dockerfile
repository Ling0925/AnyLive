# AnyLive API — multi-stage image for local/test deploy.
# Build context: backend/

FROM rust:1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --release --locked -p anylive-api \
    && strip target/release/anylive-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /app --shell /usr/sbin/nologin anylive

WORKDIR /app
COPY --from=builder /app/target/release/anylive-api /usr/local/bin/anylive-api

USER anylive
ENV API_BIND=0.0.0.0:8088 \
    RUST_LOG=info,anylive_api=info \
    APP_ENV=local

EXPOSE 8088
HEALTHCHECK --interval=10s --timeout=3s --start-period=25s --retries=12 \
  CMD curl -fsS http://127.0.0.1:8088/health || exit 1

ENTRYPOINT ["/usr/local/bin/anylive-api"]
