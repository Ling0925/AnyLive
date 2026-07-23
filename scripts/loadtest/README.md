# Load test harnesses (P1)

AnyLive separates **control plane** (Rust Axum HTTP) from **room realtime**
(Centrifugo). Do not open 1k room WebSockets against Axum.

## Scripts

| Script | Purpose |
|---|---|
| `ws-centrifugo-load.py` | **Live** Centrifugo same-room load: N WS clients + chat fan-out measure |
| `ws-1k-baseline.sh` | P1 gate harness: dry-run report stub + optional live health + optional k6 HTTP smoke |
| `gift-tps-baseline.sh` | Gift burst gate stub (100 TPS / 1min) — dry-run report + live health probe |
| `http-smoke.js` | k6 HTTP smoke against `/health` and `/meta` |
| `../dogfood-cohort-seed.sh` | Synthetic 20 hosts / 500 users control-plane seed (OTP-paced) |

## Live Centrifugo load (measured)

Requires API + Centrifugo up (`docker compose` stack). Chat is rate-limited
**5 msgs / 10s per user** — the script spaces publishes (`CHAT_PUBLISH_GAP=2.1`)
and retries on 429.

```bash
# warm-up
WS_CLIENTS=50 CHAT_MSGS=5 HOLD_SECS=25 ./scripts/loadtest/ws-centrifugo-load.py

# P1-scale connect gate (1000 clients); short observe window
WS_CLIENTS=1000 CHAT_MSGS=5 HOLD_SECS=45 ./scripts/loadtest/ws-centrifugo-load.py

# longer soak (still not a substitute for ops 15-min OBS on stage)
WS_CLIENTS=1000 CHAT_MSGS=5 HOLD_SECS=900 ./scripts/loadtest/ws-centrifugo-load.py
```

Writes `reports/ws-1k-baseline-<stamp>.md`. JWT connect auto-subscribes to
`room:{id}` (channels claim) — clients must not re-subscribe.

## 1k same-room WS (full procedure / dry-run)

1. Start deps: `docker compose -f deploy/docker-compose.yml up -d centrifugo redis`
2. Run API with Centrifugo env from `.env.example`
3. Prefer `ws-centrifugo-load.py` for measured numbers; or manual:
   - Host creates + starts a room (or `./scripts/dogfood-api-smoke.sh`)
   - `POST /api/v1/realtime/token` → connect Centrifugo WS with JWT
   - Publish chat via REST; measure connect / loss / P95
4. Archive `reports/ws-1k-baseline-*.md`

## Offline / CI

```bash
./scripts/loadtest/ws-1k-baseline.sh          # dry-run, writes reports/
./scripts/loadtest/gift-tps-baseline.sh       # gift TPS gate stub
```

No k6 or Centrifugo required for dry-run. Targets come from
`docs/product/04-非功能与容量.md`.

Filled-report templates (copy after live runs):

- `reports/ws-1k-baseline-TEMPLATE.md`
- `reports/device-matrix-TEMPLATE.md`
- `reports/backup-restore-TEMPLATE.md`
- `reports/incident-tabletop-TEMPLATE.md`
