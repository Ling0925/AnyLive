# P1 Implementation Status

Last updated: 2026-07-22

Scope: [mvp-scope.md](./mvp-scope.md). Status from `git log` + current tree (API routes, crates, clients). Backend dual-store work is owned separately; this checklist reflects what is already in tree.

---

## Implemented (with tests / feature commits)

| Area | Status | Notes |
|---|---|---|
| Monorepo + docker-compose (PG/Redis/NATS/Centrifugo/SRS/MinIO) | Done | `deploy/docker-compose.yml` |
| OpenAPI contracts + CI | Done | `contracts/`, GHA |
| Rust Axum health / meta / ready | Done | `/health`, `/ready`, `/api/v1/meta` |
| Email OTP + JWT access/refresh + `/me` + logout | Done | `feat(auth)` |
| `PATCH /me` profile (display name + age/privacy flags) | Done | dual-store via `AnyProfileExtras` + migration `003_profile_extras.sql` |
| Rooms lifecycle + SRS MediaProvider publish/play | Done | `feat(rooms)` |
| Wallet double-entry ledger + gift catalog + idempotent gifts | Done | `GET /api/v1/wallet/ledger` |
| Gift receiver guard + stricter idempotency (key reuse ≠ params) | Done | `be00695`, `b685207` |
| Chat history + Centrifugo connection token | Done | `feat(realtime,admin)` + PG dual store `AnyChat` (`cc77de0`) |
| Centrifugo HTTP publish on chat/gifts (env-gated; noop default) | Done | `publisher_from_env` |
| Admin ban / force-close / audit | Done | moderation crate |
| Admin mute / unmute (blocks chat + gifts) | Done | `POST /api/v1/admin/mute\|unmute` |
| Admin gift catalog upsert + reports queue resolve | Done | admin gifts/reports routes |
| Social follow / unfollow + following list | Done | `feat(social)` |
| Hot + following live feeds | Done | `feat(feed)` |
| User reports API | Done | `POST /api/v1/reports` |
| Postgres schema 001–006 | Done | through `006_wallet_topup_idempotency.sql` + OTP challenges |
| Postgres dual store (users/rooms/wallet/social/moderation/reports/chat/profile_extras/**deleted_users**/**refresh_tokens**/**otp_challenges**) | Done | `USE_POSTGRES=1`; all dual stores including OTP |
| SRS on_publish / on_unpublish webhooks | Done | signed HMAC publish keys; header-only webhook secret |
| Production secret guards (OTP / JWT / SRS webhook / feature flags) | Done | `ALLOW_DEV_OTP`, `ALLOW_MOCK_TOPUP`, `OTP_NOTIFIER`, `SRS_WEBHOOK_SECRET` |
| OTP hash-at-rest + notifier port + IP rate limit | Done | peppered SHA-256; `OtpNotifier`; per-IP send/verify limit |
| Moderation fail-closed + audited bootstrap | Done | try_* paths; atomic bootstrap grant |

| Compliance stubs: legal privacy/terms, account export, soft-delete | Done | API + Flutter `ComplianceRepository` |
| Chat rate limit (5 / 10s) | Done | `ChatRateLimiter` |
| Live-only gifts + public active gift catalog | Done | `ROOM_NOT_LIVE` / filter active |
| Flutter login + privacy/terms + age declaration gate | Done | login requires 18+ before Verify; best-effort PATCH /me |
| Flutter rooms / gifts / profile / feed / follow / report / ended banner | Done | Discover + room control-plane (`b28fc68`) |
| Flutter go-live OBS publish dialog + copy HLS for external player | Done | `room_list_page` publish info; room page copy stream URL |
| Flutter in-app HLS stream preview scaffolding | Done | `StreamPreview` + `hls_player_logic` (URL stage/copy/ended; media_kit embed still open) |
| H5 HLS watch + share deep-link + room-ended UI | Done | `hlsAttach` + `share` |
| H5 optional login + room chat + gifts + mock topup | Done | session in localStorage; 8s status poll; chat/gifts gated when not live |

| Admin-web dark ops console (sidebar modules) | Done | login + dashboard/rooms/reports/gifts/moderation/audit |
| Production CORS restriction | Done | `CORS_ALLOWED_ORIGINS` required when `APP_ENV=production` |
| 1k WS loadtest harness (dry-run) | Done | `scripts/loadtest/ws-1k-baseline.sh` + report stub (full Centrifugo run still operator) |
| Docker test deploy (API + Admin) | Done | `./scripts/deploy-test.sh` → API `:8088`, Admin `:8090` |
| Media dogfood smoke automation | Done | `dogfood-media-smoke.sh` + `media_smoke_lib.py` (22 unit tests) |
| Local go-live stack (compose + dogfood flags + runbook) | Done | `./scripts/deploy-test.sh` + `docs/runbooks/go-live-local.md`; dogfood env flags in compose |
| Signed publish stream key → matching HLS play path | Done | `SrsMediaProvider` active stream map; clear on stop / force-close / on_unpublish |

---

## Partial / stub (usable, not dogfood-complete)

| Item | Gap |
|---|---|
| Account export / delete | Soft-delete dual store (`AnyDeletedUsers` + `004_auth_sessions.sql`); export payload still a stub |
| Centrifugo publish | Wired; needs real Centrifugo URL/secret for live fan-out |
| Admin UI | Dark ops shell with modules; not full Vben suite |
| Flutter player | StreamPreview scaffolding shipped; media_kit / video_player embed still open |
| OTP delivery | Notifier port + hash store done; real SMTP/SMS provider still open (use `OTP_NOTIFIER=log` or fixed OTP via `ALLOW_DEV_OTP`) |
| Top-up | Mock topup only behind `ALLOW_MOCK_TOPUP=1` (no Stripe/payment provider) |

| SRS webhooks in local conf | `deploy/srs/srs.conf` has `http_hooks` → API `:8088` (host.docker.internal) |

---

## Remaining for full P1 dogfood exit

1. Live OBS push week on operator machines (stack + smoke + runbook ready: `./scripts/deploy-test.sh`, `docs/runbooks/go-live-local.md`; play URLs track signed stream keys). Control-plane dogfood is green; multi-client byte-plane still manual.
2. Real email OTP provider (notifier port exists; wire SMTP/HTTP; dual store + hash done)
3. Flutter media_kit / video_player embed (StreamPreview scaffolding + external copy-URL path shipped)
4. Full Centrifugo 1k WS run with filled report numbers + device smoke matrix
5. Full Vben admin modules if required beyond current dark ops shell

Harness for (1): `docs/runbooks/go-live-local.md`, `./scripts/dogfood-media-smoke.sh`, `scripts/dogfood-media.md`.  
Harness for (4): `./scripts/loadtest/ws-1k-baseline.sh` (dry-run) and README under `scripts/loadtest/`.

---

## MVP acceptance checklist (from mvp-scope)

### Function

- [x] Register/login OTP (dev) + browse feeds + room chat/gifts (API/Flutter/H5)
- [x] H5 watch + share + ended state
- [x] H5 optional login + chat send + gifts + mock topup
- [x] Follow host + report room (Flutter)
- [x] Admin ban/mute/force-close/gifts/reports/preview
- [x] Same idempotency key gift does not double-charge (unit/API covered)
- [x] Gifts only when room is live
- [ ] Host OBS publish dogfood week with multi-client play

### Quality

- [ ] No open P0
- [x] Wallet/gift automated tests green
- [x] 1k WS harness + report stub archived path (`scripts/loadtest/`, `reports/` gitignored)
- [ ] Full 1k Centrifugo numbers filled in a report
- [ ] Mid Android + recent iPhone smoke

### Compliance hooks

- [x] Privacy / terms visible on login (static URLs; legal API stubs available)
- [x] Age declaration on login + profile (API + UI checkboxes)
- [x] Report API + Flutter room report dialog
- [x] Account delete / export API stubs + mobile client + docs

---

## How to run

```bash
# Full local go-live stack (API :8088, Admin :8090, SRS, dogfood smokes)
./scripts/deploy-test.sh
# Guide: docs/runbooks/go-live-local.md
# SKIP_DOGFOOD_SMOKE=1 to skip smokes after up

cp .env.example .env
# docker is optional for memory-mode API dogfood
docker compose -f deploy/docker-compose.yml up -d   # optional deps (PG/Redis/…)
docker compose -f deploy/docker-compose.yml up -d srs   # only if doing OBS/HLS

cd backend && cargo test --workspace
cargo run -p anylive-api   # :8088  dev OTP = 123456

# Control-plane happy path (API must already be running)
./scripts/dogfood-api-smoke.sh
# Media path: scripts/dogfood-media.md
./scripts/dogfood-media-smoke.sh   # health + optional SRS + publish/play consistency

# Optional Postgres dual store
USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
  cargo run -p anylive-api

cd apps/mobile && flutter test
cd apps/admin-web && pnpm test
cd apps/h5-web && pnpm test
python3 -m unittest discover -s scripts -p 'test_*.py'
```
