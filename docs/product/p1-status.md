# P1 Implementation Status

Last updated: 2026-07-21

## Done (with unit tests + git commits)

| Feature | Commit theme | Tests |
|---|---|---|
| Docs + roadmap | docs: architecture ADR | n/a |
| Compose infra | chore(deploy) | smoke-compose |
| OpenAPI v0 | feat(contracts) | validate-contracts.py |
| API health | feat(backend) scaffold | cargo |
| Flutter/Vue shells | feat(apps) | flutter/vitest |
| CI | ci: GHA | — |
| Auth OTP/JWT | feat(auth) | 24 auth + API flow |
| Rooms + MediaProvider | feat(rooms) | media + rooms + API |
| Wallet + gifts | feat(wallet) | ledger idempotency + HTTP |
| Chat + Centrifugo token | feat(realtime,admin) | realtime + chat HTTP |
| Admin ban/force-close | feat(realtime,admin) | moderation + admin HTTP |
| Social follow | feat(social) | social unit |
| Hardening (ban enforce, gift key, admin bootstrap) | feat(social)+review | cargo workspace |
| Flutter login UI | feat Flutter login | widget + repo tests |
| SQL schema 001 | feat migration | db crate |

## Final review verdict

**APPROVE_WITH_FIXES** (fixes applied): ban on AuthUser, admin bootstrap lock, gift idempotency per-sender.

## Remaining for full MVP dogfood

1. SQLx wire-up to Postgres (schema ready in `backend/migrations/001_init.sql`)
2. Flutter room list / room page / player / gift panel
3. H5 watch page with hls.js
4. Admin Vben modules (not just shell)
5. Centrifugo publish on chat (not only token + memory history)
6. SRS on_publish webhook auth
7. OpenAPI yaml export sync with all routes
8. Production env guards (no fixed OTP / default JWT secrets)

## How to run

```bash
cd backend && cargo test --workspace && cargo run -p anylive-api
# OTP dev: 123456
```
