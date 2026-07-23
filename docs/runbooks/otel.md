# Observability scaffold (WBS E0.5)

## Already in-process

| Signal | Endpoint / knob | Notes |
|---|---|---|
| Health | `GET /health` | Liveness |
| Ready | `GET /ready` | Readiness (store/media flags) |
| Metrics | `GET /metrics` | Prometheus text (rooms/wallet/events/features) |
| Logs | `RUST_LOG` | `tracing` + `EnvFilter` (default `info,anylive_api=debug`) |
| Structured logs | `RUST_LOG_FORMAT=json` | JSON lines for collectors |
| HTTP spans | tower-http `TraceLayer` | Per-request spans on every route |

## Not in-repo (ops / collector)

Full **OpenTelemetry OTLP** export (Jaeger / Tempo / Honeycomb) is intentionally **not** wired into the binary yet:

1. Choose a collector (Grafana Alloy, otel-collector, cloud agent).
2. Scrape `GET /metrics` on the API pod / sidecar.
3. Ship JSON logs (`RUST_LOG_FORMAT=json`) via Fluent Bit / Vector / agent.
4. When ready for distributed traces, add `opentelemetry` + OTLP exporter crates behind `OTEL_EXPORTER_OTLP_ENDPOINT` (feature flag) without changing route semantics.

## Local check

```bash
# human logs (default)
cargo run -p anylive-api

# JSON logs
RUST_LOG_FORMAT=json cargo run -p anylive-api

curl -sS localhost:8088/metrics | head
curl -sS localhost:8088/health
```

## Stage checklist

- [ ] Prometheus scrape job for `/metrics`
- [ ] Log shipper for `RUST_LOG_FORMAT=json`
- [ ] Alert rules from `docs/runbooks/slo-alerts.md`
- [ ] Optional: OTLP endpoint + sampling rate documented in env
