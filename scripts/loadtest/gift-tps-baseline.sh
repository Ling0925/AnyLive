#!/usr/bin/env bash
# Gift burst smoke / dry-run for non-functional gate (docs/product/04).
# Full 100 TPS needs a live API + wallet seed; dry-run writes report stub only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPORT_DIR="${REPORT_DIR:-$ROOT/reports}"
mkdir -p "$REPORT_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$REPORT_DIR/gift-tps-baseline-${STAMP}.md"
API_BASE="${API_BASE:-http://127.0.0.1:8088}"
TARGET_TPS="${TARGET_TPS:-100}"
DURATION_SEC="${DURATION_SEC:-60}"
MODE="${1:-dry-run}"

cat >"$OUT" <<EOF
# Gift TPS baseline

- generated: ${STAMP}
- mode: ${MODE}
- api_base: ${API_BASE}
- target: ${TARGET_TPS} TPS for ${DURATION_SEC}s (docs/product/04 L2)
- gate: ledger balanced after burst; no double-spend on client_request_id

## Procedure (live)

1. \`cargo run -p anylive-api\` (or deploy-test) with mock topup allowed.
2. Seed N wallets via \`POST /api/v1/wallet/topups\` (admin grant or mock topup).
3. Create + start a live room; note \`room_id\` + host \`owner_id\`.
4. For each virtual sender: OTP login → topup → POST \`/api/v1/rooms/{id}/gifts\`
   with unique \`client_request_id\` (retry same id must not double debit).
5. Measure: success rate, P95 latency, wallet reconcile imbalance_count == 0.
6. Fill results below and archive.

## Results (fill when live)

| metric | value |
|---|---|
| achieved_tps | _TBD_ |
| success_rate | _TBD_ |
| p95_ms | _TBD_ |
| double_spend_detected | _TBD_ |
| reconcile_imbalance_count | _TBD_ |
| notes | |

## Dry-run note

This script only materializes the report template. Wire k6/vegeta against the
gift path in a later ops pass; control-plane correctness is covered by
\`cargo test -p anylive-api\` gift idempotency + dogfood smoke.
EOF

echo "Wrote $OUT"
if [[ "$MODE" == "dry-run" ]]; then
  exit 0
fi

# Optional health probe when not dry-run
if curl -sf "${API_BASE}/health" >/dev/null; then
  echo "API healthy at ${API_BASE} (full burst driver not automated yet)"
else
  echo "API not reachable at ${API_BASE}; report stub only" >&2
  exit 1
fi
