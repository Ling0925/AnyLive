#!/usr/bin/env bash
# Baseline harness notes + dry-run for P1 "1k same-room WS" pressure gate.
#
# Architecture rule: room fan-out lives on Centrifugo, NOT Axum. This script
# does NOT open 1000 sockets against the Rust API. It:
#   1) documents the target SLO (from docs/product/04-非功能与容量.md)
#   2) verifies control-plane readiness (health/meta/realtime token path)
#   3) optionally runs a small k6 script if k6 is installed
#   4) writes a markdown report stub under reports/
#
# Full 1k CCU requires a running Centrifugo + k6 WS scenario against Centrifugo
# channels (see scripts/loadtest/README.md). Offline CI only needs this harness
# to be present and the dry-run path to succeed without k6/Centrifugo.
#
# Usage:
#   ./scripts/loadtest/ws-1k-baseline.sh           # dry-run + report stub
#   API_BASE=http://127.0.0.1:8088 ./scripts/loadtest/ws-1k-baseline.sh --live
#   RUN_K6=1 ./scripts/loadtest/ws-1k-baseline.sh --live

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

API_BASE="${API_BASE:-http://localhost:8088}"
API_BASE="${API_BASE%/}"
REPORT_DIR="${REPORT_DIR:-$ROOT/reports}"
LIVE=0
for arg in "$@"; do
  case "$arg" in
    --live) LIVE=1 ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
  esac
done

mkdir -p "$REPORT_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
REPORT="$REPORT_DIR/ws-1k-baseline-$STAMP.md"

echo "=== AnyLive 1k WS baseline harness ==="
echo "API_BASE=$API_BASE"
echo "mode=$([ "$LIVE" = 1 ] && echo live || echo dry-run)"

if [[ "$LIVE" = 1 ]]; then
  code="$(curl -sS -o /tmp/anylive-health.json -w '%{http_code}' "$API_BASE/health" || true)"
  if [[ "$code" != "200" ]]; then
    echo "FAIL: GET $API_BASE/health -> $code (is anylive-api running?)" >&2
    exit 1
  fi
  echo "OK: health $code"
  code="$(curl -sS -o /tmp/anylive-meta.json -w '%{http_code}' "$API_BASE/api/v1/meta" || true)"
  if [[ "$code" != "200" ]]; then
    echo "FAIL: GET $API_BASE/api/v1/meta -> $code" >&2
    exit 1
  fi
  echo "OK: meta $code"
else
  echo "SKIP: live health checks (pass --live against a running API)"
fi

K6_STATUS="not-run"
if [[ "${RUN_K6:-0}" = "1" ]] && command -v k6 >/dev/null 2>&1; then
  echo "Running k6 HTTP smoke (not full 1k WS — see loadtest README)..."
  k6 run --quiet "$ROOT/scripts/loadtest/http-smoke.js" || {
    echo "WARN: k6 http-smoke failed" >&2
    K6_STATUS="failed"
  }
  if [[ "$K6_STATUS" != "failed" ]]; then
    K6_STATUS="http-smoke-ok"
  fi
elif [[ "${RUN_K6:-0}" = "1" ]]; then
  echo "WARN: RUN_K6=1 but k6 not installed"
  K6_STATUS="k6-missing"
else
  echo "SKIP: k6 (set RUN_K6=1 to attempt)"
fi

cat >"$REPORT" <<EOF
# 1k WS room pressure — baseline report stub

Generated: $STAMP (UTC)
API_BASE: \`$API_BASE\`
Mode: $([ "$LIVE" = 1 ] && echo live || echo dry-run)
k6: $K6_STATUS

## Target (P1 exit, docs/product/04-非功能与容量.md)

- Same-room **1000** WebSocket connections stable **15 min**
- Chat message loss rate **< 0.1%**
- Plane under test: **Centrifugo** (room channel), not Axum

## Architecture reminder

AnyLive hard rule: do **not** terminate 1k+ room WS connections inside the Rust
control plane. Clients obtain a Centrifugo JWT via \`POST /api/v1/realtime/token\`
and connect to Centrifugo. HTTP chat history remains on the API for reconnect
snapshots.

## Procedure (full run — operator)

1. \`docker compose -f deploy/docker-compose.yml up -d centrifugo redis\`
2. Export \`CENTRIFUGO_URL\` / \`CENTRIFUGO_API_KEY\` / \`CENTRIFUGO_TOKEN_SECRET\`
3. \`cargo run -p anylive-api\`
4. Create a live room (host OTP → create → start)
5. Issue N tokens via realtime/token; connect N WS clients to Centrifugo channel \`room:{id}\`
6. Publish chat at target rate; measure connect success, drop rate, end-to-end latency
7. Attach numbers below and commit this report under \`reports/\`

## Results (fill after full run)

| Metric | Target | Actual | Notes |
|---|---|---|---|
| Concurrent WS | 1000 | _TBD_ | Centrifugo |
| Duration | 15 min | _TBD_ | |
| Message loss | <0.1% | _TBD_ | |
| Chat E2E P95 | ≤500ms | _TBD_ | |
| API P95 during load | ≤300ms | _TBD_ | control plane |

## Dry-run status

- Harness script present: yes
- Live health: $([ "$LIVE" = 1 ] && echo checked || echo skipped)
- k6: $K6_STATUS

## Conclusion

_This file is a **baseline stub** until a full Centrifugo 1k run is executed and
numbers are filled in. Presence of the harness satisfies the offline planning
gate; archived results still required for P1 dogfood exit._
EOF

echo "Wrote $REPORT"
echo "WS_1K_BASELINE_OK"
