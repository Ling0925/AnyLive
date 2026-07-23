# H5 Watch UX — YouTube Live alignment (RoomWatch mirror)

> **Product:** AnyLive (overseas show-live)  
> **Surface:** `apps/h5-web` (Vue 3 + Vite, **single-page**, no router)  
> **Companion:** [flutter-youtube-ux.md](./flutter-youtube-ux.md)  
> **Status:** UX-1 visual/IA · 2026-07-23  
> **Scope:** Dark watch chrome + shared tokens only — **no new APIs**, no Shorts, no new PK UI

---

## 1. Goal

Mirror Flutter **RoomWatch** information architecture in H5 so mobile APK and web share one product feel:

| Priority | Element |
|---|---|
| 1 | 16:9 player stage (black) above the fold |
| 2 | Meta (LIVE · title · watching · like) |
| 3 | Channel row (host chip · more / share) |
| 4 | Live chat |
| 5 | Gift dock (horizontal pills + balance) |

Guest watch stays optional (login is not forced). Login / privacy / search / room UUID tools stay as overlays or collapsible helpers — not primary chrome.

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
header.topbar          brand · LIVE chip (canWatch) · auth
[details util]         room UUID load · share · search (collapsed by default)
[login / privacy]      overlays when toggled
.watch-layout
  .primary-col
    .player-stage      video | room-ended | room-offline | placeholder
    .meta-row          LIVE · title · watching · like  (#room-stats)
    .channel-row       host chip · ⋯ creator | Share
  .side-col            (stacks under primary on <900px)
    .chat-panel        msg list + composer
    .gift-dock         sticky bottom (mobile) · horizontal .gift-bar
```

Desktop `≥900px`: CSS grid `1fr | 340px` with sticky side column.

---

## 4. Preserved contracts

**`data-testid` (do not rename)**  
`login-panel`, `verify-otp`, `age-confirmed`, `privacy-accepted`, `privacy-panel`, `export-data`, `delete-account`, `export-json`, `search-panel`, `pk-banner`, `creator-panel`, `room-ended`, `room-offline`.

**Must not**

- Vue Router / multi-page SPA  
- New backend APIs  
- Shorts UI / new PK controls  
- Force-login for guest watch  

---

## 5. Acceptance (H5 UX-1)

- [x] Open room UUID → player stage dominates above fold  
- [x] Meta + channel visible without scrolling past chat first  
- [x] Chat + gifts usable without leaving page  
- [x] Offline / ended testids still queryable  
- [x] Shared surfaces `#0F0F0F` / `#212121` / stage black; LIVE red; CTA magenta  
- [x] English primary UI strings for end/offline  

### UX-2 / empty-shell fix (solo · 2026-07-23)

- [x] Cold open without `?room=` shows **Live now** hot feed (not player-only void)  
- [x] Room tools `<details>` auto-open when no room; collapses after load  
- [x] Meta / channel / chat / gift dock bind to `hasRoom` (not only `canWatch`)  
- [x] Gift catalog + chat history load for known rooms even if HLS not ready  
- [x] Room title from API in meta row  

---

## 6. Files

| Path | Role |
|---|---|
| `apps/h5-web/src/style.css` | Shared token `:root` |
| `apps/h5-web/src/App.vue` | RoomWatch template + scoped layout |
| `apps/h5-web/src/lib/chatApi.ts` | Paths/parsers incl. `feedHotPath` |
| `apps/h5-web/index.html` | `theme-color` `#0F0F0F`, title |
| `docs/product/flutter-youtube-ux.md` | Flutter counterpart |

Build: `cd apps/h5-web && VITE_API_BASE=http://localhost:8088 pnpm build`
