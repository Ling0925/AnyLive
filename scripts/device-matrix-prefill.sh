#!/usr/bin/env bash
# Scaffold a dated device-matrix report from the template.
#
# Creates reports/device-matrix-YYYYMMDD-<device>.md with:
#   - empty path cells (Login/Feed/HLS/Chat/Gift) — never pre-filled pass
#   - adb serial / OS stubs for the operator to fill
#   - explicit "Pass = not claimed" footer
#
# This script NEVER signs V-FL-1 and NEVER claims Pass.
#
# Usage (from repo root):
#   ./scripts/device-matrix-prefill.sh mid-android
#   ./scripts/device-matrix-prefill.sh android-23116PN5BC
#   ./scripts/device-matrix-prefill.sh iphone-15
#   ./scripts/device-matrix-prefill.sh h5-safari
#   DEVICE_SLUG=mid-android ADB_SERIAL=R58M... OS_STUB="Android 12" ./scripts/device-matrix-prefill.sh
#
# Env (optional):
#   DEVICE_SLUG   — same as first arg
#   ADB_SERIAL    — prefill adb serial stub (Android); default empty
#   OS_STUB       — prefill OS cell stub; default empty
#   DATE_STAMP    — override YYYYMMDD (default: local date)
#   FORCE=1       — overwrite existing report file
#   DRY_RUN=1     — print path only, do not write

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SLUG="${1:-${DEVICE_SLUG:-}}"
if [[ -z "$SLUG" || "$SLUG" == "-h" || "$SLUG" == "--help" ]]; then
  echo "usage: $0 <device-slug>" >&2
  echo "  e.g. mid-android | android-23116PN5BC | iphone-15 | h5-safari | h5-chrome" >&2
  echo "  env: DEVICE_SLUG ADB_SERIAL OS_STUB DATE_STAMP FORCE=1 DRY_RUN=1" >&2
  echo "  never auto-signs V-FL-1; path cells start empty; Pass=not claimed" >&2
  exit 2
fi

# sanitize slug: lowercase, spaces→-, strip unsafe chars
SLUG="$(printf '%s' "$SLUG" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9._-')"
if [[ -z "$SLUG" ]]; then
  echo "FAIL: device slug empty after sanitize" >&2
  exit 2
fi

DATE_STAMP="${DATE_STAMP:-$(date +%Y%m%d)}"
OUT="reports/device-matrix-${DATE_STAMP}-${SLUG}.md"
ADB_SERIAL="${ADB_SERIAL:-}"
OS_STUB="${OS_STUB:-}"
FORCE="${FORCE:-0}"
DRY_RUN="${DRY_RUN:-0}"

TEMPLATE="reports/device-matrix-TEMPLATE.md"
if [[ ! -f "$TEMPLATE" ]]; then
  echo "FAIL: missing $TEMPLATE" >&2
  exit 1
fi

if [[ -f "$OUT" && "$FORCE" != "1" && "$FORCE" != "true" ]]; then
  echo "FAIL: $OUT already exists (set FORCE=1 to overwrite)" >&2
  exit 1
fi

# Guess which matrix row to highlight from slug
ROW_HINT="(fill the matching row; leave others N/A — not run)"
case "$SLUG" in
  *android*|*mid*) ROW_HINT="Mid Android row — fill this device only unless multi-device" ;;
  *iphone*|*ios*)  ROW_HINT="Recent iPhone row — fill this device only unless multi-device" ;;
  *safari*)        ROW_HINT="H5 Safari row" ;;
  *chrome*)        ROW_HINT="H5 Chrome row" ;;
  *h5*)            ROW_HINT="H5 browser row (Safari or Chrome — pick one)" ;;
esac

# Human label for Device cell (not a pass claim)
DEVICE_LABEL="$SLUG"
case "$SLUG" in
  mid-android) DEVICE_LABEL="Mid Android" ;;
  *iphone*)    DEVICE_LABEL="Recent iPhone ($SLUG)" ;;
  h5-safari)   DEVICE_LABEL="H5 Safari" ;;
  h5-chrome)   DEVICE_LABEL="H5 Chrome" ;;
esac

TODAY_HUMAN="$(date +%Y-%m-%d)"

if [[ "$DRY_RUN" == "1" || "$DRY_RUN" == "true" ]]; then
  echo "DRY_RUN would write: $OUT"
  exit 0
fi

mkdir -p reports

cat >"$OUT" <<EOF
# 设备矩阵冒烟 — ${DEVICE_LABEL}

Date (local): ${TODAY_HUMAN}
Source template: \`reports/device-matrix-TEMPLATE.md\`
Scaffolded by: \`scripts/device-matrix-prefill.sh\`
Scope: **${ROW_HINT}** — path cells start **empty**; install stubs only. **not** a full critical-path smoke, not OBS week dogfood.

## Meta

| Field | Value |
|---|---|
| Operator | |
| App build | |
| API base | |
| adb serial | ${ADB_SERIAL} |
| OS / browser | ${OS_STUB} |
| Report file | \`${OUT}\` |

## Matrix

| Device | OS | Build | Login | Feed | HLS play | Chat | Gift | Crash |
|---|---|---|---|---|---|---|---|---|
| Mid Android |  |  |  |  |  |  |  |  |
| Recent iPhone |  |  |  |  |  |  |  |  |
| H5 Safari |  |  |  |  |  |  |  |  |
| H5 Chrome |  |  |  |  |  |  |  |  |

> Prefill left **all path cells empty**. Put the device under test into the matching row; set other rows to \`N/A — not run\` only after you decide scope.
> Allowed path values: \`pass\` · \`fail\` · \`blocked\` · \`not run\` (see template).

## Critical path checklist (this device only)

- [ ] Login
- [ ] Feed
- [ ] HLS play
- [ ] Chat
- [ ] Gift
- [ ] Crash review (\`none observed\` / defect id)

## Pass line

Pass = all critical paths green, no P0 crash.

- [ ] **Pass claimed by operator** (date / name): ________________
- [x] **Pass = not claimed.** This file was scaffolded empty. Automation and prefill **must never** auto-sign V-FL-1.

## Notes

- Operator: replace stubs; record only exercised steps.
- Launch success ≠ path smoke.
- Remaining open for P1 quality exit until required rows + Pass checkbox are human-filled.

## Related

- Template: \`reports/device-matrix-TEMPLATE.md\`
- API base for emu / adb reverse / LAN: \`apps/mobile/README.md\`, \`apps/mobile/store/README.md\`
- H5 rows: \`apps/h5-web/README.md\`
- V-FL-1 / V-FL-2: \`docs/product/p1-parallel-tracks.md\`, \`docs/runbooks/dogfood-cohort.md\`
EOF

echo "Wrote $OUT"
echo "Pass is NOT claimed. Fill path cells manually; do not mark V-FL-1 done from this scaffold alone."
