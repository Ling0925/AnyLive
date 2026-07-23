#!/usr/bin/env bash
# Automated 10-minute dogfood control-plane path (host + viewer + gift).
#
# Flow:
#   1. Host OTP → create room → start → media/publish → print OBS + HLS
#   2. Viewer OTP → feed/hot → get room → chat → topup/mock pay → send gift
#      twice with same client_request_id (assert idempotent, no double debit)
#   3. Optional admin force-close
#
# Exit 0 only if critical steps pass. Control-plane only (no real OBS push).
# Does NOT close V-BE-1/2, plan 06 exit #1/#2, or sign risk-accept docs.
#
# Prerequisites:
#   cargo run -p anylive-api  OR  ./scripts/deploy-test.sh
#   Local: APP_ENV=local / ALLOW_DEV_OTP=1, ALLOW_MOCK_TOPUP=1 for gifts.
#
# Usage:
#   ./scripts/dogfood-10min-path.sh
#   API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 ./scripts/dogfood-10min-path.sh
#   DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=https://api.stage.example ./scripts/dogfood-10min-path.sh
#   # OBS week: leave room live so you can paste credentials into OBS
#   SKIP_FORCE_CLOSE=1 ./scripts/dogfood-10min-path.sh
#
# Env (shared with dogfood-api-smoke.sh):
#   API_BASE, OTP_CODE, DOGFOOD_STRICT, DOGFOOD_ADMIN_EMAIL,
#   DOGFOOD_PG_CONTAINER, POSTGRES_USER, POSTGRES_DB
# Optional:
#   SKIP_FORCE_CLOSE=1  — skip admin force-close (leave room live; recommended for OBS week)
#   ALLOW_P3_FEATURES=1 — allow FEATURE_PK / FEATURE_COHOST on (default: FAIL if on)
#   DOGFOOD_REPORT_DIR  — if set, tee full stdout/stderr to $DIR/dogfood-10min-path-<UTC>.log

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"

export OTP_CODE="${OTP_CODE:-123456}"
API_BASE="${API_BASE:-http://localhost:8088}"
API_BASE="${API_BASE%/}"
OTP_CODE="${OTP_CODE:-123456}"
DOGFOOD_STRICT="${DOGFOOD_STRICT:-0}"
SKIP_FORCE_CLOSE="${SKIP_FORCE_CLOSE:-0}"
ALLOW_P3_FEATURES="${ALLOW_P3_FEATURES:-0}"

# Optional log tee for redeploy evidence (does not change exit semantics).
if [[ -n "${DOGFOOD_REPORT_DIR:-}" ]]; then
  mkdir -p "$DOGFOOD_REPORT_DIR"
  _df_log="${DOGFOOD_REPORT_DIR}/dogfood-10min-path-$(date -u +%Y%m%dT%H%M%SZ).log"
  exec > >(tee "$_df_log") 2>&1
  echo "logging to ${_df_log}"
fi

HOST_EMAIL="dogfood-10m-host-$(date +%s)-$$@example.com"
VIEWER_EMAIL="dogfood-10m-viewer-$(date +%s)-$$@example.com"

step=0
FAILED=0

skip_mock() {
  [[ "$DOGFOOD_STRICT" = "1" || "$DOGFOOD_STRICT" = "true" ]]
}

obs_server_from_push() {
  # Derive OBS Server from push_url (not hardcoded localhost).
  PYTHONPATH="${SCRIPTS_DIR}${PYTHONPATH:+:$PYTHONPATH}" python3 -c \
    "import sys; from media_smoke_lib import obs_server_from_push_url; print(obs_server_from_push_url(sys.argv[1], sys.argv[2]))" \
    "$1" "$2"
}

print_obs_block() {
  local server="$1" stream_key="$2" push_url="$3" hls="${4:-}" flv="${5:-}" expires="${6:-}"
  echo
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║  OBS — paste-ready (custom RTMP)                             ║"
  echo "╠══════════════════════════════════════════════════════════════╣"
  echo "║  Server:      ${server}"
  echo "║  Stream Key:  ${stream_key}"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo "  push_url:   ${push_url}"
  [[ -n "$expires" ]] && echo "  expires_at: ${expires}"
  if [[ -n "$hls" ]]; then
    echo "┌──────────────────────────────────────────────────────────────┐"
    echo "│  HLS (H5 / Flutter / VLC):                                   │"
    echo "│  ${hls}"
    [[ -n "$flv" ]] && echo "│  flv: ${flv}"
    echo "└──────────────────────────────────────────────────────────────┘"
  fi
  if [[ "$stream_key" != *"?"* ]] || [[ "$stream_key" != *"exp="* ]] || [[ "$stream_key" != *"sig="* ]]; then
    echo "WARN: stream_key should include ?exp=&sig= (signed token). Bare room UUID will be rejected by on_publish." >&2
  else
    echo "NOTE: stream_key MUST be pasted whole, including ?exp=&sig= (not bare room UUID)."
  fi
}

print_human_obs_checklist() {
  local room_id="$1"
  echo
  echo "======== Human OBS week checklist (control-plane green ≠ exit signed) ========"
  echo "  [ ] 1. OBS → Settings → Stream → Service=Custom"
  echo "  [ ] 2. Paste Server + full Stream Key (with ?exp=&sig=) → Start Streaming"
  echo "  [ ] 3. H5 (?room=${room_id}) and/or Flutter room page plays HLS"
  echo "  [ ] 4. Stop OBS / unpublish — room leaves live (webhook or host stop)"
  echo "  Tip (OBS week): re-run with SKIP_FORCE_CLOSE=1 so this script leaves the room live."
  echo "  This PASS does NOT close V-BE-1, V-BE-2, plan 06 exit #1/#2, or risk-accept drafts."
  echo "  See: docs/runbooks/go-live-local.md · scripts/dogfood-media.md"
  echo "=============================================================================="
}

label() {
  step=$((step + 1))
  echo
  echo "=== [$step] $* ==="
}

api() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local token="${4:-}"
  local url="${API_BASE}${path}"
  local args=(-sS -X "$method" "$url" -w $'\n%{http_code}' -H "Accept: application/json")
  if [[ -n "$token" ]]; then
    args+=(-H "Authorization: Bearer ${token}")
  fi
  if [[ -n "$body" ]]; then
    args+=(-H "Content-Type: application/json" -d "$body")
  fi
  local raw
  raw="$(curl "${args[@]}")"
  HTTP_STATUS="${raw##*$'\n'}"
  HTTP_BODY="${raw%$'\n'*}"
  if [[ ! "$HTTP_STATUS" =~ ^2[0-9][0-9]$ ]]; then
    echo "FAIL: ${method} ${path} -> HTTP ${HTTP_STATUS}" >&2
    echo "Body: ${HTTP_BODY}" >&2
    exit 1
  fi
}

api_soft() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local token="${4:-}"
  local url="${API_BASE}${path}"
  local args=(-sS -X "$method" "$url" -w $'\n%{http_code}' -H "Accept: application/json")
  if [[ -n "$token" ]]; then
    args+=(-H "Authorization: Bearer ${token}")
  fi
  if [[ -n "$body" ]]; then
    args+=(-H "Content-Type: application/json" -d "$body")
  fi
  local raw
  raw="$(curl "${args[@]}")"
  HTTP_STATUS="${raw##*$'\n'}"
  HTTP_BODY="${raw%$'\n'*}"
}

json_get() {
  local json="$1"
  local expr="$2"
  python3 -c "import json,sys; obj=json.loads(sys.argv[1]); print(${expr})" "$json"
}

ensure_admin() {
  ADMIN_TOKEN=""
  api_soft POST /api/v1/admin/grant "{\"user_id\":\"${HOST_ID}\"}" "$HOST_TOKEN"
  if [[ "$HTTP_STATUS" =~ ^2[0-9][0-9]$ ]]; then
    echo "admin bootstrap grant: ok (HTTP ${HTTP_STATUS})"
    ADMIN_TOKEN="$HOST_TOKEN"
    return 0
  fi
  echo "admin bootstrap grant: HTTP ${HTTP_STATUS} (${HTTP_BODY}) — will try alternate admin path"

  if [[ -n "${DOGFOOD_ADMIN_EMAIL:-}" ]]; then
    local admin_email="$DOGFOOD_ADMIN_EMAIL"
    api POST /api/v1/auth/otp/send "{\"email\":\"${admin_email}\"}"
    api POST /api/v1/auth/otp/verify "{\"email\":\"${admin_email}\",\"code\":\"${OTP_CODE}\"}"
    ADMIN_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
    echo "using DOGFOOD_ADMIN_EMAIL=${admin_email}"
    return 0
  fi

  local pg_container="${DOGFOOD_PG_CONTAINER:-anylive-postgres-1}"
  local pg_user="${POSTGRES_USER:-anylive}"
  local pg_db="${POSTGRES_DB:-anylive}"
  if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' | grep -qx "$pg_container"; then
    echo "seeding admin_users for host via docker exec ${pg_container}"
    docker exec -i "$pg_container" \
      psql -U "$pg_user" -d "$pg_db" -v ON_ERROR_STOP=1 \
      -c "INSERT INTO admin_users (user_id) VALUES ('${HOST_ID}') ON CONFLICT DO NOTHING;" \
      >/dev/null
    docker exec -i "$pg_container" psql -U "$pg_user" -d "$pg_db" -c \
      "UPDATE admin_users SET role = 'admin' WHERE user_id = '${HOST_ID}'::uuid;" \
      >/dev/null 2>&1 || true
    ADMIN_TOKEN="$HOST_TOKEN"
    api_soft GET /api/v1/admin/wallet/reconcile "" "$ADMIN_TOKEN"
    if [[ "$HTTP_STATUS" =~ ^2[0-9][0-9]$ ]]; then
      echo "admin seed verified (reconcile HTTP ${HTTP_STATUS})"
      return 0
    fi
    echo "WARN: admin seed did not unlock reconcile (HTTP ${HTTP_STATUS}: ${HTTP_BODY})" >&2
  fi

  echo "FAIL: no admin path available for force-close." >&2
  echo "Set DOGFOOD_ADMIN_EMAIL or run ./scripts/seed-admin-local.sh ops@example.com" >&2
  return 1
}

echo "AnyLive dogfood 10-minute control-plane path"
echo "API_BASE=${API_BASE}  OTP_CODE=${OTP_CODE}  DOGFOOD_STRICT=${DOGFOOD_STRICT}"
echo "SKIP_FORCE_CLOSE=${SKIP_FORCE_CLOSE}  ALLOW_P3_FEATURES=${ALLOW_P3_FEATURES}"
echo "NOTE: control-plane only — does not close V-BE-1/2 or sign risk-accept docs."

# ---------------------------------------------------------------------------
label "P1-safe feature guard (GET /api/v1/meta)"
api GET /api/v1/meta
META_PK="$(json_get "$HTTP_BODY" "obj.get('features',{}).get('pk')")"
META_COHOST="$(json_get "$HTTP_BODY" "obj.get('features',{}).get('cohost')")"
echo "features.pk=${META_PK} features.cohost=${META_COHOST}"
pk_on=0
cohost_on=0
[[ "$META_PK" = "True" || "$META_PK" = "true" || "$META_PK" = "1" ]] && pk_on=1
[[ "$META_COHOST" = "True" || "$META_COHOST" = "true" || "$META_COHOST" = "1" ]] && cohost_on=1
if [[ "$pk_on" -eq 1 || "$cohost_on" -eq 1 ]]; then
  if [[ "$ALLOW_P3_FEATURES" = "1" || "$ALLOW_P3_FEATURES" = "true" ]]; then
    echo "WARN: FEATURE_PK/COHOST on but ALLOW_P3_FEATURES=1 — continuing (not P1 default)"
  else
    echo "FAIL: FEATURE_PK/FEATURE_COHOST must be off for default dogfood (P1-safe)." >&2
    echo "  features.pk=${META_PK} features.cohost=${META_COHOST}" >&2
    echo "  Set FEATURE_PK=0 FEATURE_COHOST=0 on the API, or ALLOW_P3_FEATURES=1 to soft-allow." >&2
    echo "  PK/cohost are P3 experimental — never Wave2 DoD." >&2
    exit 1
  fi
else
  echo "P1-safe: pk/cohost off (expected)"
fi

# ---------------------------------------------------------------------------
label "Host OTP send + verify (${HOST_EMAIL})"
api POST /api/v1/auth/otp/send "{\"email\":\"${HOST_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${HOST_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
HOST_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
HOST_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "host_id=${HOST_ID}"

# ---------------------------------------------------------------------------
label "Host creates room + starts live"
api POST /api/v1/rooms "{\"title\":\"Dogfood 10min Path\"}" "$HOST_TOKEN"
ROOM_ID="$(json_get "$HTTP_BODY" "obj['id']")"
ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "room_id=${ROOM_ID} status=${ROOM_STATUS}"
api POST "/api/v1/rooms/${ROOM_ID}/start" "" "$HOST_TOKEN"
ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "after start: status=${ROOM_STATUS}"
if [[ "$ROOM_STATUS" != "live" ]]; then
  echo "FAIL: expected room status live, got ${ROOM_STATUS}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Host media/publish (OBS fields)"
api POST "/api/v1/rooms/${ROOM_ID}/media/publish" "" "$HOST_TOKEN"
PUSH_URL="$(json_get "$HTTP_BODY" "obj['push_url']")"
STREAM_KEY="$(json_get "$HTTP_BODY" "obj['stream_key']")"
EXPIRES_AT="$(json_get "$HTTP_BODY" "obj.get('expires_at','')")"
OBS_SERVER="$(obs_server_from_push "$PUSH_URL" "$STREAM_KEY")"
if [[ -z "$OBS_SERVER" ]]; then
  echo "WARN: could not derive OBS server from push_url; falling back to path strip" >&2
  OBS_SERVER="$(python3 -c "u='''${PUSH_URL}'''.strip(); i=u.rfind('/'); print(u[:i] if i>0 else u)")"
fi

# ---------------------------------------------------------------------------
label "Media/play (HLS for viewers)"
api GET "/api/v1/rooms/${ROOM_ID}/media/play"
HLS_URL="$(json_get "$HTTP_BODY" "obj['hls']")"
FLV_URL="$(json_get "$HTTP_BODY" "obj.get('flv','')")"
echo "hls=${HLS_URL}"
echo "flv=${FLV_URL}"

print_obs_block "$OBS_SERVER" "$STREAM_KEY" "$PUSH_URL" "$HLS_URL" "$FLV_URL" "$EXPIRES_AT"

# ---------------------------------------------------------------------------
label "Viewer OTP send + verify (${VIEWER_EMAIL})"
api POST /api/v1/auth/otp/send "{\"email\":\"${VIEWER_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${VIEWER_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
VIEWER_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
VIEWER_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "viewer_id=${VIEWER_ID}"

# ---------------------------------------------------------------------------
label "Viewer feed/hot + get room"
api GET /api/v1/feed/hot
HOT_COUNT="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
echo "hot_live_count=${HOT_COUNT}"
if [[ "$HOT_COUNT" -lt 1 ]]; then
  echo "FAIL: expected hot feed to include live room" >&2
  exit 1
fi
api GET "/api/v1/rooms/${ROOM_ID}"
GET_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "get room: status=${GET_STATUS}"
if [[ "$GET_STATUS" != "live" ]]; then
  echo "FAIL: expected room live, got ${GET_STATUS}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Viewer posts chat message"
api POST "/api/v1/rooms/${ROOM_ID}/messages" \
  "{\"body\":\"hello from dogfood 10min viewer\"}" \
  "$VIEWER_TOKEN"
MSG_ID="$(json_get "$HTTP_BODY" "obj['id']")"
echo "message_id=${MSG_ID}"
api GET "/api/v1/rooms/${ROOM_ID}/messages"
MSG_COUNT="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
echo "message_count=${MSG_COUNT}"
if [[ "$MSG_COUNT" -lt 1 ]]; then
  echo "FAIL: expected at least one chat message" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Viewer topup / mock pay (coins for gift)"
if ! skip_mock; then
  api POST /api/v1/wallet/topups \
    "{\"amount\":1000,\"reference\":\"dogfood-10m-topup-$$\"}" \
    "$VIEWER_TOKEN"
  BAL="$(json_get "$HTTP_BODY" "obj['balance']")"
  echo "viewer balance after mock topup=${BAL}"
else
  echo "SKIP mock topup (DOGFOOD_STRICT=1)"
  api GET /api/v1/wallet "" "$VIEWER_TOKEN"
  BAL="$(json_get "$HTTP_BODY" "obj['balance']")"
  echo "viewer balance=${BAL}"
fi

# ---------------------------------------------------------------------------
label "List gifts + send with idempotency key (twice)"
api GET /api/v1/gifts
GIFT_COUNT="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
if [[ "$GIFT_COUNT" -lt 1 ]]; then
  echo "FAIL: gift catalog empty — run ./scripts/dogfood-gift-seed.sh first" >&2
  exit 1
fi
# Prefer cheapest gift for dogfood (price ascending).
GIFT_ID="$(json_get "$HTTP_BODY" "sorted(obj['items'], key=lambda g: g.get('price', 0))[0]['id']")"
GIFT_NAME="$(json_get "$HTTP_BODY" "sorted(obj['items'], key=lambda g: g.get('price', 0))[0]['name']")"
GIFT_PRICE="$(json_get "$HTTP_BODY" "sorted(obj['items'], key=lambda g: g.get('price', 0))[0]['price']")"
echo "gift=${GIFT_NAME} id=${GIFT_ID} price=${GIFT_PRICE}"

if skip_mock; then
  echo "SKIP gift send (DOGFOOD_STRICT=1 — no mock topup guarantee)"
else
  api GET /api/v1/wallet "" "$VIEWER_TOKEN"
  BAL_BEFORE="$(json_get "$HTTP_BODY" "obj['balance']")"
  CLIENT_REQ="dogfood-10m-gift-$(date +%s)-$$"
  GIFT_BODY="{\"gift_id\":\"${GIFT_ID}\",\"receiver_id\":\"${HOST_ID}\",\"count\":1,\"client_request_id\":\"${CLIENT_REQ}\"}"

  api POST "/api/v1/rooms/${ROOM_ID}/gifts" "$GIFT_BODY" "$VIEWER_TOKEN"
  GIFT_ORDER="$(json_get "$HTTP_BODY" "obj['id']")"
  REPLAYED1="$(json_get "$HTTP_BODY" "obj['replayed']")"
  TOTAL1="$(json_get "$HTTP_BODY" "obj['total_coins']")"
  echo "gift_order=${GIFT_ORDER} replayed=${REPLAYED1} total_coins=${TOTAL1}"
  if [[ "$REPLAYED1" != "False" && "$REPLAYED1" != "false" ]]; then
    echo "FAIL: first gift send should not be replayed" >&2
    exit 1
  fi

  # Replay same client_request_id — must not double-debit.
  api POST "/api/v1/rooms/${ROOM_ID}/gifts" "$GIFT_BODY" "$VIEWER_TOKEN"
  GIFT_ORDER2="$(json_get "$HTTP_BODY" "obj['id']")"
  REPLAYED2="$(json_get "$HTTP_BODY" "obj['replayed']")"
  echo "gift replay: order=${GIFT_ORDER2} replayed=${REPLAYED2}"
  if [[ "$REPLAYED2" != "True" && "$REPLAYED2" != "true" ]]; then
    echo "FAIL: second gift send with same client_request_id must be replayed" >&2
    exit 1
  fi
  if [[ "$GIFT_ORDER2" != "$GIFT_ORDER" ]]; then
    echo "FAIL: replay must return same order id (${GIFT_ORDER} vs ${GIFT_ORDER2})" >&2
    exit 1
  fi

  api GET /api/v1/wallet "" "$VIEWER_TOKEN"
  BAL_AFTER="$(json_get "$HTTP_BODY" "obj['balance']")"
  echo "viewer balance before=${BAL_BEFORE} after=${BAL_AFTER} (expect debit once of ${TOTAL1})"
  python3 -c "
import sys
b, a, t = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
assert a == b - t, f'double-debit or wrong amount: before={b} after={a} total={t}'
" "$BAL_BEFORE" "$BAL_AFTER" "$TOTAL1"
  echo "idempotent gift: no double debit OK"
fi

# ---------------------------------------------------------------------------
if [[ "$SKIP_FORCE_CLOSE" = "1" || "$SKIP_FORCE_CLOSE" = "true" ]]; then
  label "Skip admin force-close (SKIP_FORCE_CLOSE=1 — leave room live for OBS)"
  echo "room left live: ${ROOM_ID}"
  echo "Tip: paste OBS block above; after human push/play, stop via OBS unpublish or host stop."
else
  label "Admin force-close room"
  echo "Tip for OBS week: re-run with SKIP_FORCE_CLOSE=1 to leave the room live."
  if ensure_admin; then
    api POST /api/v1/admin/rooms/force-close \
      "{\"room_id\":\"${ROOM_ID}\",\"reason\":\"dogfood 10min path\"}" \
      "$ADMIN_TOKEN"
    FC_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
    echo "force-close status=${FC_STATUS}"
    api GET "/api/v1/rooms/${ROOM_ID}"
    FINAL_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
    echo "get room after force-close: status=${FINAL_STATUS}"
    if [[ "$FINAL_STATUS" == "live" ]]; then
      echo "FAIL: expected non-live after force-close, got ${FINAL_STATUS}" >&2
      exit 1
    fi
  else
    echo "WARN: force-close skipped (no admin path) — host stop fallback"
    api POST "/api/v1/rooms/${ROOM_ID}/stop" "" "$HOST_TOKEN"
    ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
    echo "after host stop: status=${ROOM_STATUS}"
    FAILED=1
  fi
fi

echo
# Re-print paste-ready OBS block at the end (easy to find after long gift path).
print_obs_block "$OBS_SERVER" "$STREAM_KEY" "$PUSH_URL" "$HLS_URL" "${FLV_URL:-}" "$EXPIRES_AT"
print_human_obs_checklist "$ROOM_ID"

if [[ "$FAILED" -ne 0 ]]; then
  echo "DOGFOOD_10MIN_PATH_PARTIAL (critical path ok; admin force-close failed)"
  echo "room_id=${ROOM_ID} host=${HOST_ID} viewer=${VIEWER_ID}"
  echo "obs_server=${OBS_SERVER}"
  echo "publish=${PUSH_URL}"
  echo "stream_key=${STREAM_KEY}"
  echo "hls=${HLS_URL}"
  echo "NOTE: PARTIAL/PASS is control-plane only — not V-BE-1/2 or plan 06 #1/#2 signed."
  exit 1
fi

echo "DOGFOOD_10MIN_PATH_PASS"
echo "room_id=${ROOM_ID} host=${HOST_ID} viewer=${VIEWER_ID}"
echo "obs_server=${OBS_SERVER}"
echo "publish=${PUSH_URL}"
echo "stream_key=${STREAM_KEY}"
echo "hls=${HLS_URL}"
echo "NOTE: PASS is control-plane only — not V-BE-1/2 or plan 06 #1/#2 signed."
