# H5 Watch UX — YouTube Live alignment (Home + RoomWatch)

> **Product:** AnyLive (overseas show-live)  
> **Surface:** `apps/h5-web` (Vue 3 + Vite, **single-page**, no router)  
> **Companion:** [flutter-youtube-ux.md](./flutter-youtube-ux.md)  
> **Status:** Home + RoomWatch · 2026-07-23  
> **Scope:** Dark discover + watch chrome + shared tokens — **no Vue Router**, no new APIs, no Shorts, no new PK UI

---

## 1. Goal

Two surfaces in one SPA (view switch, not a router):

| View | When | Purpose |
|---|---|---|
| **Home** | cold open, no `?room=`, or after “← Home” | Discover: hot feed cards + search |
| **Watch** | `?room=<uuid>` deep link, or card / UUID open | RoomWatch: player + meta + chat + gifts |

Guest watch stays optional (login is not forced). Login / privacy stay overlays.

---

## 2. Shared visual tokens

Aligned with Flutter `AnyColors` (see plan table in flutter-youtube-ux / UX-1):

| Token | Value | CSS var |
|---|---|---|
| `bg.app` | `#0F0F0F` | `--bg` |
| `bg.elevated` | `#212121` | `--bg-elevated` |
| `bg.player` | `#000000` | `--bg-stage` |
| `bg.input` | `#121212` | `--bg-input` |
| `text.primary` | `#F1F1F1` | `--text` |
| `text.secondary` | `#AAAAAA` | `--text-muted` |
| `accent` (CTA) | `#C850FF` | `--accent` |
| `accent.soft` | `rgba(200,80,255,0.15)` | `--accent-soft` |
| `live` | `#FF0033` | `--live` |
| `success` | `#3DDC97` | `--success` |
| `danger` | `#FF4D4F` | `--danger` |
| `radius.card` | `12` | `--radius-md` |
| `radius.pill` | `999` | `--radius-pill` |

**Rules**

- Dark-only; no light theme.
- LIVE badge = red (`--live`) + white/red text; **never magenta**.
- Primary buttons / filled CTAs = brand magenta (`--accent`).
- No Shorts rail; no PK promotion (banner only when `featurePk && pk`, de-emphasized).

Source of truth in code: `apps/h5-web/src/style.css` `:root`.

---

## 3. Layout wireframe (single `main.page`)

```
header.topbar          brand(→Home) · Home nav · LIVE(watch) · auth
[login / privacy]      overlays when toggled

── view = home ──
.home-view
  hero “Live now”
  search row
  .home-grid           LiveCards (16:9 thumb + title + LIVE)
  details tools        paste UUID

── view = watch ──
.watch-toolbar         ← Home · room tools
.watch-layout
  .primary-col
    .player-stage      video | room-ended | room-offline | placeholder
    .meta-row          LIVE · title · watching · like  (#room-stats)
    .channel-row       host chip · ⋯ creator | Share
  .side-col            (stacks under primary on <900px)
    .chat-panel
    .gift-dock
```

Desktop `≥900px` (watch only): CSS grid `1fr | 340px` with sticky side column.

Deep link: `?room=<uuid>` → enter watch + `history.replaceState` keeps URL in sync; Home clears `room` query.

---

## 4. Preserved contracts

**`data-testid` (do not rename)**  
`login-panel`, `verify-otp`, `age-confirmed`, `privacy-accepted`, `privacy-panel`, `export-data`, `delete-account`, `export-json`, `search-panel`, `pk-banner`, `creator-panel`, `room-ended`, `room-offline`.

**Added (Home / nav)**  
`home-view`, `watch-view`, `hot-feed`, `nav-home`, `back-home`, `room-id-input`, `load-room`.

**Must not**

- Vue Router / multi-page SPA  
- New backend APIs  
- Shorts UI / new PK controls  
- Force-login for guest watch  
- Auto-open a room as the “home” landing  

---

## 5. Acceptance

### UX-1 RoomWatch

- [x] Open room UUID / card → player stage dominates above fold  
- [x] Meta + channel visible without scrolling past chat first  
- [x] Chat + gifts usable without leaving page  
- [x] Offline / ended testids still queryable  
- [x] Shared surfaces `#0F0F0F` / `#212121` / stage black; LIVE red; CTA magenta  
- [x] English primary UI strings for end/offline  

### Home + empty-shell fix (solo · 2026-07-23)

- [x] Cold open **is Home** (hot grid), not a room / player shell  
- [x] Card click / search / UUID → Watch; `?room=` deep link still works  
- [x] Top brand + **← Home** leave watch and clear room state / query  
- [x] Meta / channel / chat / gift dock bind to `hasRoom` (not only `canWatch`)  
- [x] Gift catalog + chat history load for known rooms even if HLS not ready  
- [x] Room title from API in meta row  

---

## 6. Files

| Path | Role |
|---|---|
| `apps/h5-web/src/style.css` | Shared token `:root` |
| `apps/h5-web/src/App.vue` | Home + RoomWatch views (no router) |
| `apps/h5-web/src/lib/chatApi.ts` | Paths/parsers incl. `feedHotPath` |
| `apps/h5-web/index.html` | `theme-color` `#0F0F0F`, title |
| `docs/product/flutter-youtube-ux.md` | Flutter counterpart |

Build: `cd apps/h5-web && VITE_API_BASE=http://localhost:8088 pnpm build`
