# Load test harnesses (P1)

AnyLive separates **control plane** (Rust Axum HTTP) from **room realtime**
(Centrifugo). Do not open 1k room WebSockets against Axum.

## Scripts

| Script | Purpose |
|---|---|
| `ws-1k-baseline.sh` | P1 gate harness: dry-run report stub + optional live health + optional k6 HTTP smoke |
| `http-smoke.js` | k6 HTTP smoke against `/health` and `/meta` |

## 1k same-room WS (full procedure)

1. Start deps: `docker compose -f deploy/docker-compose.yml up -d centrifugo redis`
2. Run API with Centrifugo env from `.env.example`
3. Host creates + starts a room (or `./scripts/dogfood-api-smoke.sh`)
4. For each virtual user: `POST /api/v1/realtime/token` with `{ "room_id": "..." }`
5. Connect WS clients to Centrifugo with that token; subscribe to `room:{id}`
6. Publish chat (REST `POST .../messages` or Centrifugo publish path) and measure:
   - connect success rate
   - message loss over 15 minutes
   - end-to-end latency P95
7. Fill numbers into the generated `reports/ws-1k-baseline-*.md` and archive

## Offline / CI

```bash
./scripts/loadtest/ws-1k-baseline.sh          # dry-run, writes reports/
```

No k6 or Centrifugo required for dry-run. Targets come from
`docs/product/04-非功能与容量.md`.
