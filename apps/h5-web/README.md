# AnyLive H5 watch page

Public **Vue 3 + Vite** audience page (hls.js / native HLS). **Not** Flutter Web.

Stack defaults: API on `:8088`, optional Centrifugo WS. Local dogfood: `docs/runbooks/go-live-local.md`.

## Prerequisites

- Node 20+ / pnpm
- Reachable API (`./scripts/deploy-test.sh` or local `anylive-api`)

## Env

| Variable | Default | Role |
|---|---|---|
| `VITE_API_BASE` | `http://localhost:8088` | API origin (no trailing slash required) |
| `VITE_CENTRIFUGO_WS` | empty | optional Centrifugo WS; empty → HTTP chat history poll |

Used in `src/App.vue` as `import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'`.

Phone browser on the same LAN should use the **host LAN IP**, not `localhost`:

```bash
VITE_API_BASE=http://192.168.1.20:8088 pnpm dev
```

## Develop

```bash
cd apps/h5-web
pnpm install
pnpm dev
# open http://localhost:5173/?room=<room-uuid>
```

Optional OpenAPI client regen (shared script):

```bash
pnpm gen:api          # write generated types
pnpm check:api        # check-only (CI-style)
```

## Static build + preview

```bash
cd apps/h5-web

# local test stack
VITE_API_BASE=http://localhost:8088 pnpm build
pnpm preview
# default preview: http://localhost:4173/

# stage-shaped static build
VITE_API_BASE=https://api.stage.example.com pnpm build
```

| Script | Command | Notes |
|---|---|---|
| `dev` | `vite` | HMR |
| `build` | `vue-tsc -b && vite build` | output `dist/` |
| `preview` | `vite preview` | serve `dist/` for smoke |
| `test` | `vitest run` | unit only — not a substitute for browser matrix |

Serve `dist/` behind any static host in stage/prod; inject `VITE_*` **at build time** (Vite bakes them in).

## Watch path (manual)

1. Start stack; host goes live (Admin / Flutter / `dogfood-10min-path.sh`).
2. `GET /api/v1/rooms/{id}/media/play` → HLS URL.
3. Open H5 with `?room=<uuid>`.
4. Login if exercising chat/gift (dev OTP `123456` on local).
5. Confirm player, chat poll/WS, gift (sandbox) as needed.

## V-FL-1 — H5 Safari / H5 Chrome matrix rows

Device matrix template: `reports/device-matrix-TEMPLATE.md`.  
Scaffold a browser-only report (path cells empty; **Pass not claimed**):

```bash
# from repo root
./scripts/device-matrix-prefill.sh h5-safari
./scripts/device-matrix-prefill.sh h5-chrome
```

### Procedure (operator)

| Step | Action |
|---|---|
| 1 | Control-plane green optional but recommended: `./scripts/dogfood-10min-path.sh` PASS (leaves a live room + HLS). |
| 2 | Build or dev-serve H5 with a **browser-reachable** `VITE_API_BASE` (localhost on desktop; LAN IP from a phone). |
| 3 | Open the page in **Safari** (macOS or iOS) and/or **Chrome** — one report per browser if separate files. |
| 4 | Fill matrix cells only for steps **actually** exercised: Login · Feed (or direct `?room=`) · HLS play · Chat · Gift · Crash. |
| 5 | Leave unused rows as `N/A — not run`. Do **not** check Pass unless all required H5 rows you own are green. |
| 6 | Prefill footer stays **Pass = not claimed** until a human signs. Never auto-sign V-FL-1. |

Cell vocabulary: `pass` / `fail` / `blocked` / `not run` (see template).

Native Android/iOS rows are covered under `apps/mobile/README.md` + the same matrix template.

## Related

- Mobile install / `API_BASE_URL`: `apps/mobile/README.md`, `apps/mobile/store/README.md`
- Dogfood + V-FL-2 recording checklist: `docs/runbooks/dogfood-cohort.md`
- Parallel tracks board: `docs/product/p1-parallel-tracks.md`
