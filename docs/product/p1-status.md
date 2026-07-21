# P1 Implementation Status

Last updated: 2026-07-21

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
| Postgres schema 001–003 | Done | `backend/migrations/` (`001_init`, `002_reports_mute`, `003_profile_extras`) |
| Postgres dual store (users/rooms/wallet/social/moderation/reports/**chat**/profile_extras) | Done | `USE_POSTGRES=1`; soft-delete + refresh tokens still process-local |
| SRS on_publish / on_unpublish webhooks | Done | `routes/webhooks.rs` |
| Production secret guards (OTP / JWT / etc.) | Done | API startup guards |
| Compliance stubs: legal privacy/terms, account export, soft-delete | Done | API + Flutter `ComplianceRepository` |
| Chat rate limit (5 / 10s) | Done | `ChatRateLimiter` |
| Live-only gifts + public active gift catalog | Done | `ROOM_NOT_LIVE` / filter active |
| Flutter login + privacy/terms URLs | Done | `login_page.dart` |
| Flutter rooms / gifts / profile / feed / follow / report / ended banner | Done | Discover + room control-plane (`b28fc68`) |
| H5 HLS watch + share deep-link + room-ended UI | Done | `hlsAttach` + `share` |
| Admin-web OTP + moderation + gifts + reports + HLS preview | Done | `admin.ts` + `App.vue` |
| Control-plane dogfood smoke script | Done | `scripts/dogfood-api-smoke.sh` |
| Media dogfood notes (OBS → SRS → H5/Flutter) | Done | `scripts/dogfood-media.md` |

---

## Partial / stub (usable, not dogfood-complete)

| Item | Gap |
|---|---|
| Account export / delete | Soft-delete is process-local `DeletedUsers` HashSet; not multi-replica / not PG |
| Refresh tokens | `InMemoryRefreshStore` even when Postgres dual store is on |
| Centrifugo publish | Wired; needs real Centrifugo URL/secret for live fan-out |
| H5 | Watch+share only; no login / chat / gifts |
| Admin UI | Functional shell, not full Vben module suite |
| Flutter player | Room page control-plane; HLS URL only (no media_kit embed) |
| OTP delivery | Dev fixed code path; no real email provider |
| Top-up | Mock topup only (no Stripe/payment provider) |
| SRS webhooks in local conf | Handler exists; local `srs.conf` may still need `http_hooks` pointed at API |

---

## Remaining for full P1 dogfood exit

1. End-to-end OBS → SRS → Flutter/H5 HLS play smoke on compose stack (control-plane script is ready; media path documented in `scripts/dogfood-media.md`)
2. Real email OTP (or documented test harness for dogfood — dev code `123456` is the current harness)
3. Persist soft-delete + refresh tokens (or document single-process dogfood)
4. In-app Flutter player (or documented external player path for hosts — see media dogfood notes)
5. 1k WS room pressure report + device smoke matrix
6. Full Vben admin modules if required beyond current shell

---

## MVP acceptance checklist (from mvp-scope)

### Function

- [x] Register/login OTP (dev) + browse feeds + room chat/gifts (API/Flutter)
- [x] H5 watch + share + ended state
- [x] Follow host + report room (Flutter)
- [x] Admin ban/mute/force-close/gifts/reports/preview
- [x] Same idempotency key gift does not double-charge (unit/API covered)
- [x] Gifts only when room is live
- [ ] Host OBS publish dogfood week with multi-client play

### Quality

- [ ] No open P0
- [x] Wallet/gift automated tests green
- [ ] 1k WS report archived
- [ ] Mid Android + recent iPhone smoke

### Compliance hooks

- [x] Privacy / terms visible on login (static URLs; legal API stubs available)
- [x] Age declaration on profile (API + profile UI checkboxes)
- [x] Report API + Flutter room report dialog
- [x] Account delete / export API stubs + mobile client + docs

---

## How to run

```bash
cp .env.example .env
# docker is optional for memory-mode API dogfood
docker compose -f deploy/docker-compose.yml up -d   # optional deps (PG/Redis/…)
docker compose -f deploy/docker-compose.yml up -d srs   # only if doing OBS/HLS

cd backend && cargo test --workspace
cargo run -p anylive-api   # :8088  dev OTP = 123456

# Control-plane happy path (API must already be running)
./scripts/dogfood-api-smoke.sh
# Media path notes: scripts/dogfood-media.md

# Optional Postgres dual store
USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
  cargo run -p anylive-api

cd apps/mobile && flutter test
cd apps/admin-web && pnpm test
cd apps/h5-web && pnpm test
```
