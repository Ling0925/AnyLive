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
| `PATCH /me` profile (display name + age/privacy flags) | Done | process-local extras |
| Rooms lifecycle + SRS MediaProvider publish/play | Done | `feat(rooms)` |
| Wallet double-entry ledger + gift catalog + idempotent gifts | Done | `GET /api/v1/wallet/ledger` |
| Gift receiver guard + stricter idempotency (key reuse ≠ params) | Done | `be00695`, `b685207` |
| Chat history + Centrifugo connection token | Done | `feat(realtime,admin)` |
| Centrifugo HTTP publish on chat/gifts (env-gated; noop default) | Done | `publisher_from_env` |
| Admin ban / force-close / audit | Done | moderation crate |
| Admin mute / unmute (blocks chat + gifts) | Done | `POST /api/v1/admin/mute\|unmute` |
| Admin gift catalog upsert + reports queue resolve | Done | admin gifts/reports routes |
| Social follow / unfollow + following list | Done | `feat(social)` |
| Hot + following live feeds | Done | `feat(feed)` |
| User reports API | Done | `POST /api/v1/reports` |
| Postgres schema `001_init.sql` | Done | `backend/migrations/` |
| Postgres dual store (users/rooms/wallet via `USE_POSTGRES=1`) | Done | `anylive-db` + `Any*` stores |
| SRS on_publish / on_unpublish webhooks | Done | `routes/webhooks.rs` |
| Production secret guards (OTP / JWT / etc.) | Done | API startup guards |
| Compliance stubs: legal privacy/terms, account export, soft-delete | Done | API + Flutter `ComplianceRepository` |
| Flutter login shell + privacy/terms URLs | Done | `login_page.dart` + widget test |
| Flutter rooms repository, room list, room page (chat/gifts shell) | Done | control-plane UI |
| Flutter gifts repository + wallet ledger client | Done | `listLedger()` |
| H5 watch shell + hls.js attach (native HLS fallback) | Done | `hlsAttach.ts` |
| Admin-web OTP login shell + ban/mute/force-close/gifts/reports | Done | `admin.ts` + `App.vue` |

---

## Partial / stub (usable, not dogfood-complete)

| Item | Gap |
|---|---|
| Account export / delete | Soft-delete + stub payload; no full DSAR package / hard purge |
| Age / privacy profile extras | In-memory only (`MemoryProfileExtras`); not in PG migration |
| Moderation / social / reports / chat bus | Process-local memory even when Postgres dual store is on |
| Centrifugo publish | Wired; needs real Centrifugo URL/secret for live fan-out |
| H5 | Watch-only; no login / chat / gifts |
| Admin UI | Functional shell, not full Vben module suite |
| Flutter player | Room page is control-plane (chat/gifts); no in-app HLS player |
| OTP delivery | Dev fixed code path; no real email provider |
| Top-up | Mock topup only (no Stripe/payment provider) |

---

## Remaining for full P1 dogfood exit

1. End-to-end OBS → SRS → Flutter/H5 HLS play smoke on compose stack  
2. Real email OTP (or documented test harness for dogfood)  
3. Persist moderation / social / reports / chat when `USE_POSTGRES=1`  
4. OpenAPI YAML sync with all mounted routes (compliance, mute, ledger, webhooks, feeds, PATCH me)  
5. In-app Flutter player (or documented external player path for hosts)  
6. H5 login + chat/gift optional path (MVP allows watch+share only)  
7. Age declaration UI on registration (API extras exist; client field incomplete)  
8. Report entry on Flutter room UI (API exists)  
9. 1k WS room pressure report + device smoke matrix  
10. Full Vben admin modules if required beyond current shell  

---

## MVP acceptance checklist (from mvp-scope)

### Function

- [ ] New user 10 min: register → watch → chat → gift (test coins)  
- [ ] Host OBS publish; Flutter + H5 can play  
- [ ] Stop / force-close yields clear ended state on clients  
- [x] Same idempotency key gift does not double-charge (unit/API covered)  
- [x] Admin ban / mute / force-close APIs with audit (enforcement wired)

### Quality

- [ ] No open P0  
- [x] Wallet/gift automated tests green  
- [ ] 1k WS report archived  
- [ ] Mid Android + recent iPhone smoke

### Compliance hooks

- [x] Privacy / terms visible on login (static URLs; legal API stubs available)  
- [ ] Age declaration field in registration UI  
- [x] Report API available (UI entry incomplete)  
- [x] Account delete / export API stubs + mobile client + docs

---

## How to run

```bash
cp .env.example .env
docker compose -f deploy/docker-compose.yml up -d   # optional deps

cd backend && cargo test --workspace
cargo run -p anylive-api   # :8088  dev OTP = 123456

# Optional Postgres dual store
USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
  cargo run -p anylive-api

cd apps/mobile && flutter test
cd apps/admin-web && pnpm test
cd apps/h5-web && pnpm test
```
