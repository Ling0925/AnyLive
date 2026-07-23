#!/usr/bin/env bash
# Control-plane dogfood smoke against a running AnyLive API.
#
# Prerequisites:
#   cargo run -p anylive-api   # local: fixed OTP + set ALLOW_MOCK_TOPUP=1 for gifts
# Memory mode needs no docker. Optional Postgres dual store:
#   USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
#     cargo run -p anylive-api
#
# Usage:
#   ./scripts/dogfood-api-smoke.sh
#   API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 ./scripts/dogfood-api-smoke.sh
#   DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=https://api.stage.example ./scripts/dogfood-api-smoke.sh
#
# Requires: curl, python3. Fails on non-2xx (except documented 204).

set -euo pipefail

# Dogfood expects local API with fixed OTP + mock topup.
# Server side: APP_ENV=local (or ALLOW_DEV_OTP=1) and ALLOW_MOCK_TOPUP=1 for gifts.
# These exports only affect this script process, not the API server.
export OTP_CODE="${OTP_CODE:-123456}"

API_BASE="${API_BASE:-http://localhost:8088}"
OTP_CODE="${OTP_CODE:-123456}"
API_BASE="${API_BASE%/}"
# Stage/prod-ish: DOGFOOD_STRICT=1 skips mock topup / mock pay / sandbox-complete.
# Requires a real OTP (OTP_CODE) and a server without mock channels if you assert pay.
DOGFOOD_STRICT="${DOGFOOD_STRICT:-0}"

HOST_EMAIL="dogfood-host-$(date +%s)@example.com"
FAN_EMAIL="dogfood-fan-$(date +%s)@example.com"

step=0

skip_mock() {
  [[ "$DOGFOOD_STRICT" = "1" || "$DOGFOOD_STRICT" = "true" ]]
}

label() {
  step=$((step + 1))
  echo
  echo "=== [$step] $* ==="
}

# curl wrapper: captures body + status; fails unless status is 2xx.
# Usage: api METHOD PATH [json_body] [bearer]
# Sets: HTTP_STATUS, HTTP_BODY
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

# Soft variant: never exits; caller inspects HTTP_STATUS.
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
  # json_get <json> <python expr on obj, e.g. "obj['access_token']">
  local json="$1"
  local expr="$2"
  python3 -c "import json,sys; obj=json.loads(sys.argv[1]); print(${expr})" "$json"
}

# Ensure ADMIN_TOKEN can call admin routes.
# 1) Try bootstrap grant for HOST (first-boot only).
# 2) Else login DOGFOOD_ADMIN_EMAIL if set.
# 3) Else seed HOST into admin_users via docker postgres (local test stack).
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

  # Local docker postgres seed (compose project name: anylive).
  local pg_container="${DOGFOOD_PG_CONTAINER:-anylive-postgres-1}"
  local pg_user="${POSTGRES_USER:-anylive}"
  local pg_db="${POSTGRES_DB:-anylive}"
  if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' | grep -qx "$pg_container"; then
    echo "seeding admin_users for host via docker exec ${pg_container}"
    docker exec -i "$pg_container" \
      psql -U "$pg_user" -d "$pg_db" -v ON_ERROR_STOP=1 \
      -c "INSERT INTO admin_users (user_id) VALUES ('${HOST_ID}') ON CONFLICT DO NOTHING;" \
      >/dev/null
    ADMIN_TOKEN="$HOST_TOKEN"
    # Verify admin works
    api_soft GET /api/v1/admin/wallet/reconcile "" "$ADMIN_TOKEN"
    if [[ "$HTTP_STATUS" =~ ^2[0-9][0-9]$ ]]; then
      echo "admin seed verified (reconcile HTTP ${HTTP_STATUS})"
      return 0
    fi
    echo "WARN: admin seed did not unlock reconcile (HTTP ${HTTP_STATUS}: ${HTTP_BODY})" >&2
  fi

  echo "FAIL: no admin path available." >&2
  echo "  - empty DB bootstrap grant failed (admin already exists), and" >&2
  echo "  - DOGFOOD_ADMIN_EMAIL not set, and" >&2
  echo "  - docker postgres seed unavailable/failed." >&2
  echo "Set DOGFOOD_ADMIN_EMAIL to an existing admin, or reset admin_users." >&2
  exit 1
}

echo "AnyLive dogfood API smoke"
echo "API_BASE=${API_BASE}  OTP_CODE=${OTP_CODE}"
echo "(API must already be running — docker not required for memory mode)"

# ---------------------------------------------------------------------------
label "Host OTP send + verify (${HOST_EMAIL})"
api POST /api/v1/auth/otp/send "{\"email\":\"${HOST_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${HOST_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
HOST_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
HOST_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "host_id=${HOST_ID}"

# ---------------------------------------------------------------------------
label "Fan OTP send + verify (${FAN_EMAIL})"
api POST /api/v1/auth/otp/send "{\"email\":\"${FAN_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${FAN_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
FAN_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
FAN_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "fan_id=${FAN_ID}"

# ---------------------------------------------------------------------------
label "Host creates room + starts live"
api POST /api/v1/rooms "{\"title\":\"Dogfood Room\"}" "$HOST_TOKEN"
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
label "Host media/publish (RTMP URL / stream key)"
api POST "/api/v1/rooms/${ROOM_ID}/media/publish" "" "$HOST_TOKEN"
PUSH_URL="$(json_get "$HTTP_BODY" "obj['push_url']")"
STREAM_KEY="$(json_get "$HTTP_BODY" "obj['stream_key']")"
echo "push_url=${PUSH_URL}"
echo "stream_key=${STREAM_KEY}"

# ---------------------------------------------------------------------------
label "Fan media/play (HLS URL)"
api GET "/api/v1/rooms/${ROOM_ID}/media/play"
HLS_URL="$(json_get "$HTTP_BODY" "obj['hls']")"
echo "hls=${HLS_URL}"

# ---------------------------------------------------------------------------
label "Fan topup + list gifts + send gift"
if ! skip_mock; then
api POST /api/v1/wallet/topups "{\"amount\":1000,\"reference\":\"dogfood-topup\"}" "$FAN_TOKEN"
BAL="$(json_get "$HTTP_BODY" "obj['balance']")"
echo "fan balance after topup=${BAL}"
else
  echo "SKIP mock topup (DOGFOOD_STRICT=1) — gifts send skipped if no balance"
fi
api GET /api/v1/gifts
GIFT_ID="$(json_get "$HTTP_BODY" "obj['items'][0]['id']")"
GIFT_NAME="$(json_get "$HTTP_BODY" "obj['items'][0]['name']")"
echo "gift=${GIFT_NAME} id=${GIFT_ID}"
if ! skip_mock; then
CLIENT_REQ="dogfood-gift-$(date +%s)-$$"
api POST "/api/v1/rooms/${ROOM_ID}/gifts" \
  "{\"gift_id\":\"${GIFT_ID}\",\"receiver_id\":\"${HOST_ID}\",\"count\":1,\"client_request_id\":\"${CLIENT_REQ}\"}" \
  "$FAN_TOKEN"
GIFT_ORDER="$(json_get "$HTTP_BODY" "obj['id']")"
REPLAYED="$(json_get "$HTTP_BODY" "obj['replayed']")"
echo "gift_order=${GIFT_ORDER} replayed=${REPLAYED}"
else
  echo "SKIP gift send (DOGFOOD_STRICT=1)"
fi

# ---------------------------------------------------------------------------
label "Fan posts chat message + lists messages"
api POST "/api/v1/rooms/${ROOM_ID}/messages" \
  "{\"body\":\"hello from dogfood fan\"}" \
  "$FAN_TOKEN"
MSG_ID="$(json_get "$HTTP_BODY" "obj['id']")"
echo "message_id=${MSG_ID}"
api GET "/api/v1/rooms/${ROOM_ID}/messages"
MSG_COUNT="$(json_get "$HTTP_BODY" "len(obj['items'])")"
echo "message_count=${MSG_COUNT}"
if [[ "$MSG_COUNT" -lt 1 ]]; then
  echo "FAIL: expected at least one chat message" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Fan follows host + feed/hot"
api POST "/api/v1/users/${HOST_ID}/follow" "" "$FAN_TOKEN"
api GET /api/v1/me/following "" "$FAN_TOKEN"
FOLLOWING="$(json_get "$HTTP_BODY" "obj['user_ids']")"
echo "following=${FOLLOWING}"
api GET /api/v1/feed/hot
HOT_COUNT="$(json_get "$HTTP_BODY" "len(obj['items'])")"
echo "hot_live_count=${HOT_COUNT}"
if [[ "$HOT_COUNT" -lt 1 ]]; then
  echo "FAIL: expected hot feed to include live room" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Fan reports room"
api POST /api/v1/reports \
  "{\"target_type\":\"room\",\"target_id\":\"${ROOM_ID}\",\"reason\":\"dogfood spam check\"}" \
  "$FAN_TOKEN"
REPORT_ID="$(json_get "$HTTP_BODY" "obj['id']")"
REPORT_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "report_id=${REPORT_ID} status=${REPORT_STATUS}"

# ---------------------------------------------------------------------------
if ! skip_mock; then
label "Fan pay products + mock order + sandbox-complete"
# Mock channel is enabled when PAY_CHANNELS=mock / PAY_ENABLE_MOCK=1 /
# or ALLOW_MOCK_TOPUP=1 with empty PAY_CHANNELS (dogfood default).
api GET /api/v1/pay/channels
PAY_CH_COUNT="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
echo "pay_channels=${PAY_CH_COUNT} body=${HTTP_BODY}"
api GET /api/v1/pay/products
PAY_PROD_COUNT="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
if [[ "$PAY_PROD_COUNT" -lt 1 ]]; then
  echo "FAIL: expected pay products catalog" >&2
  exit 1
fi
PRODUCT_ID="$(json_get "$HTTP_BODY" "obj['items'][0]['id']")"
PRODUCT_COINS="$(json_get "$HTTP_BODY" "obj['items'][0]['coins']")"
echo "product_id=${PRODUCT_ID} coins=${PRODUCT_COINS}"
# Record balance before pay mint
api GET /api/v1/wallet "" "$FAN_TOKEN"
BAL_BEFORE="$(json_get "$HTTP_BODY" "obj['balance']")"
CLIENT_PAY="dogfood-pay-$(date +%s)-$$"
api POST /api/v1/pay/orders \
  "{\"product_id\":\"${PRODUCT_ID}\",\"channel\":\"mock\",\"client_request_id\":\"${CLIENT_PAY}\"}" \
  "$FAN_TOKEN"
PAY_ORDER_ID="$(json_get "$HTTP_BODY" "obj['id']")"
PAY_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "pay_order=${PAY_ORDER_ID} status=${PAY_STATUS}"
api POST "/api/v1/pay/orders/${PAY_ORDER_ID}/sandbox-complete" "" "$FAN_TOKEN"
PAY_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "after sandbox-complete: status=${PAY_STATUS}"
if [[ "$PAY_STATUS" != "credited" ]]; then
  echo "FAIL: expected pay order credited, got ${PAY_STATUS}" >&2
  exit 1
fi
api GET /api/v1/wallet "" "$FAN_TOKEN"
BAL_AFTER="$(json_get "$HTTP_BODY" "obj['balance']")"
echo "fan balance before=${BAL_BEFORE} after=${BAL_AFTER}"
# Coins must increase (integer compare)
python3 -c "import sys; b=int(sys.argv[1]); a=int(sys.argv[2]); c=int(sys.argv[3]);
assert a >= b + c, (b,a,c)" "$BAL_BEFORE" "$BAL_AFTER" "$PRODUCT_COINS"

# ---------------------------------------------------------------------------
else
  echo "SKIP mock section (DOGFOOD_STRICT=1)"
fi
label "Fan account export payload"
api GET /api/v1/me/export "" "$FAN_TOKEN"
EXPORT_VER="$(json_get "$HTTP_BODY" "obj.get('schema_version','')")"
EXPORT_BAL="$(json_get "$HTTP_BODY" "obj['wallet']['balance']")"
EXPORT_USER="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "export schema=${EXPORT_VER} user=${EXPORT_USER} wallet_balance=${EXPORT_BAL}"
if [[ -z "$EXPORT_VER" ]]; then
  echo "FAIL: export missing schema_version" >&2
  exit 1
fi
if [[ "$EXPORT_USER" != "$FAN_ID" ]]; then
  echo "FAIL: export user id mismatch" >&2
  exit 1
fi

# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
label "Fan presence heartbeat + like + room stats"
api POST "/api/v1/rooms/${ROOM_ID}/presence" "" "$FAN_TOKEN"
ONLINE="$(json_get "$HTTP_BODY" "obj.get('online_count')")"
echo "online_after_fan_presence=${ONLINE}"
api POST "/api/v1/rooms/${ROOM_ID}/likes" "{}" "$FAN_TOKEN"
LIKES="$(json_get "$HTTP_BODY" "obj.get('like_count')")"
ACCEPTED="$(json_get "$HTTP_BODY" "obj.get('accepted')")"
echo "like accepted=${ACCEPTED} count=${LIKES}"
api GET "/api/v1/rooms/${ROOM_ID}/stats"
STATS_ONLINE="$(json_get "$HTTP_BODY" "obj.get('online_count')")"
STATS_LIKES="$(json_get "$HTTP_BODY" "obj.get('like_count')")"
echo "stats online=${STATS_ONLINE} likes=${STATS_LIKES}"
if [[ "${STATS_LIKES}" == "0" || -z "${STATS_LIKES}" || "${STATS_LIKES}" == "None" ]]; then
  echo "FAIL: expected like_count >= 1" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Search rooms by title"
api GET "/api/v1/search?q=Dogfood&type=rooms"
SEARCH_N="$(json_get "$HTTP_BODY" "len(obj.get('rooms') or [])")"
echo "search rooms=${SEARCH_N}"
if [[ "$SEARCH_N" -lt 1 ]]; then
  echo "FAIL: expected search rooms >= 1" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Fan sessions list (logout-all not called — would kill token)"
api GET /api/v1/me/sessions "" "$FAN_TOKEN"
SESS_N="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
echo "sessions=${SESS_N}"
if [[ "$SESS_N" -lt 1 ]]; then
  echo "FAIL: expected at least one refresh session" >&2
  exit 1
fi
SESS_JTI="$(json_get "$HTTP_BODY" "obj['items'][0]['jti']")"
echo "session_jti=${SESS_JTI}"
# Single-session revoke is destructive for that jti; re-login after to keep token usable.
# Instead, verify 404 for a random uuid (not owned / missing).
api_soft DELETE "/api/v1/me/sessions/00000000-0000-0000-0000-000000000000" "" "$FAN_TOKEN"
if [[ "$HTTP_STATUS" != "404" ]]; then
  echo "FAIL: expected 404 for unknown jti, got ${HTTP_STATUS}" >&2
  exit 1
fi
echo "single-session revoke 404 for unknown jti: ok"

# ---------------------------------------------------------------------------
label "Fan push token register + list + unregister"
api POST /api/v1/me/push-tokens \
  "{\"token\":\"dogfood-push-$$\",\"platform\":\"android\"}" "$FAN_TOKEN"
PUSH_TOK="$(json_get "$HTTP_BODY" "obj.get('token')")"
echo "push_token=${PUSH_TOK}"
api GET /api/v1/me/push-tokens "" "$FAN_TOKEN"
PUSH_N="$(json_get "$HTTP_BODY" "len(obj.get('items') or [])")"
echo "push_tokens=${PUSH_N}"
if [[ "$PUSH_N" -lt 1 ]]; then
  echo "FAIL: expected push token list >= 1" >&2
  exit 1
fi
api DELETE /api/v1/me/push-tokens \
  "{\"token\":\"dogfood-push-$$\",\"platform\":\"android\"}" "$FAN_TOKEN"
# DELETE may return 204 with empty body — api() still requires 2xx
echo "push token unregistered"

# ---------------------------------------------------------------------------
label "Avatar presign + confirm (synthetic blob URL)"
api POST /api/v1/me/avatar/presign "{\"content_type\":\"image/jpeg\"}" "$FAN_TOKEN"
AV_KEY="$(json_get "$HTTP_BODY" "obj.get('object_key')")"
AV_PUBLIC="$(json_get "$HTTP_BODY" "obj.get('public_url')")"
AV_UPLOAD="$(json_get "$HTTP_BODY" "obj.get('upload_url')")"
echo "avatar object_key=${AV_KEY}"
if [[ -z "$AV_KEY" || "$AV_KEY" == "None" ]]; then
  echo "FAIL: expected avatar object_key" >&2
  exit 1
fi
if [[ -z "$AV_UPLOAD" || "$AV_UPLOAD" == "None" ]]; then
  echo "FAIL: expected avatar upload_url" >&2
  exit 1
fi
api POST /api/v1/me/avatar/confirm \
  "{\"object_key\":\"${AV_KEY}\",\"public_url\":\"${AV_PUBLIC}\"}" "$FAN_TOKEN"
AV_ME="$(json_get "$HTTP_BODY" "obj.get('avatar_url')")"
echo "avatar_url=${AV_ME}"
if [[ -z "$AV_ME" || "$AV_ME" == "None" ]]; then
  echo "FAIL: expected avatar_url on confirm" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Host recording toggle"
api GET "/api/v1/rooms/${ROOM_ID}/recording"
REC_OFF="$(json_get "$HTTP_BODY" "obj.get('recording_enabled')")"
echo "recording initial=${REC_OFF}"
api PUT "/api/v1/rooms/${ROOM_ID}/recording" "{\"enabled\":true}" "$HOST_TOKEN"
REC_ON="$(json_get "$HTTP_BODY" "obj.get('recording_enabled')")"
echo "recording after put=${REC_ON}"
if [[ "$REC_ON" != "True" && "$REC_ON" != "true" ]]; then
  echo "FAIL: expected recording_enabled true" >&2
  exit 1
fi
api GET "/api/v1/rooms/${ROOM_ID}/stats"
REC_STATS="$(json_get "$HTTP_BODY" "obj.get('recording_enabled')")"
echo "stats recording_enabled=${REC_STATS}"
if [[ "$REC_STATS" != "True" && "$REC_STATS" != "true" ]]; then
  echo "FAIL: stats should reflect recording_enabled" >&2
  exit 1
fi
api PUT "/api/v1/rooms/${ROOM_ID}/recording" "{\"enabled\":false}" "$HOST_TOKEN"

label "Host ensure admin + wallet ledger reconcile"
ensure_admin
api GET /api/v1/admin/wallet/reconcile "" "$ADMIN_TOKEN"
RECON_BALANCED="$(json_get "$HTTP_BODY" "obj.get('balanced')")"
RECON_IMBALANCE="$(json_get "$HTTP_BODY" "obj.get('imbalance_count')")"
RECON_CHECKED="$(json_get "$HTTP_BODY" "obj.get('checked_users')")"
echo "reconcile balanced=${RECON_BALANCED} imbalance=${RECON_IMBALANCE} checked=${RECON_CHECKED}"
if [[ "$RECON_BALANCED" != "True" && "$RECON_BALANCED" != "true" ]]; then
  echo "FAIL: wallet reconcile not balanced: ${HTTP_BODY}" >&2
  exit 1
fi
if [[ "$RECON_IMBALANCE" != "0" ]]; then
  echo "FAIL: expected imbalance_count=0, got ${RECON_IMBALANCE}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Co-host invite + PK start/end + analytics events (optional P3; default off)"
# P3 surfaces default FEATURE_PK=0 / FEATURE_COHOST=0 (plan 06). Soft-skip when disabled.
api_soft POST /api/v1/auth/otp/send "{\"email\":\"dogfood-host-b@example.com\"}"
api_soft POST /api/v1/auth/otp/verify "{\"email\":\"dogfood-host-b@example.com\",\"code\":\"123456\"}"
if [[ ! "$HTTP_STATUS" =~ ^2 ]]; then
  echo "SKIP: host-b login failed HTTP ${HTTP_STATUS}"
else
  HOST_B_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
  HOST_B_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
  api_soft POST /api/v1/rooms "{\"title\":\"Dogfood PK B\"}" "$HOST_B_TOKEN"
  ROOM_B_ID="$(json_get "$HTTP_BODY" "obj['id']" 2>/dev/null || true)"
  if [[ -n "${ROOM_B_ID:-}" ]]; then
    api_soft POST "/api/v1/rooms/${ROOM_B_ID}/start" "" "$HOST_B_TOKEN"
  fi

  api_soft POST "/api/v1/rooms/${ROOM_ID}/interactive/invite" \
    "{\"invitee_id\":\"${FAN_ID}\"}" "$HOST_TOKEN"
  if [[ "$HTTP_STATUS" = "403" ]] || echo "$HTTP_BODY" | grep -qi 'feature flag\|co-host is disabled\|PK is disabled'; then
    echo "SKIP: co-host/PK disabled by FEATURE_COHOST/FEATURE_PK (expected for P1 dogfood)"
  elif [[ ! "$HTTP_STATUS" =~ ^2 ]]; then
    echo "FAIL: interactive invite -> HTTP ${HTTP_STATUS}" >&2
    echo "Body: ${HTTP_BODY}" >&2
    exit 1
  else
    INV_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
    echo "interactive invite status=${INV_STATUS}"
    if [[ "$INV_STATUS" != "invited" ]]; then
      echo "FAIL: expected invite status invited, got ${INV_STATUS}" >&2
      exit 1
    fi
    api POST "/api/v1/rooms/${ROOM_ID}/interactive/respond" \
      "{\"accept\":true}" "$FAN_TOKEN"
    INV_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
    echo "interactive after accept=${INV_STATUS}"
    if [[ "$INV_STATUS" != "active" ]]; then
      echo "FAIL: expected active co-host, got ${INV_STATUS}" >&2
      exit 1
    fi

    api_soft POST "/api/v1/rooms/${ROOM_ID}/pk/start" \
      "{\"opponent_room_id\":\"${ROOM_B_ID}\",\"duration_secs\":120}" "$HOST_TOKEN"
    if [[ "$HTTP_STATUS" = "403" ]] || echo "$HTTP_BODY" | grep -qi 'feature flag\|PK is disabled'; then
      echo "SKIP: PK disabled by FEATURE_PK (expected for P1 dogfood)"
    elif [[ ! "$HTTP_STATUS" =~ ^2 ]]; then
      echo "FAIL: pk start -> HTTP ${HTTP_STATUS}" >&2
      echo "Body: ${HTTP_BODY}" >&2
      exit 1
    else
      PK_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
      echo "pk start status=${PK_STATUS}"
      if [[ "$PK_STATUS" != "active" ]]; then
        echo "FAIL: expected active PK, got ${PK_STATUS}" >&2
        exit 1
      fi
      api POST "/api/v1/rooms/${ROOM_ID}/pk/end" "" "$HOST_TOKEN"
      PK_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
      echo "pk end status=${PK_STATUS}"
      if [[ "$PK_STATUS" != "ended" ]]; then
        echo "FAIL: expected ended PK, got ${PK_STATUS}" >&2
        exit 1
      fi
    fi
  fi
fi

api POST /api/v1/events \
  "{\"events\":[{\"name\":\"dogfood.smoke\",\"client_event_id\":\"dogfood-$$\"}]}" \
  "$FAN_TOKEN"
EV_ACCEPTED="$(json_get "$HTTP_BODY" "obj.get('accepted')")"
echo "events accepted=${EV_ACCEPTED}"
if [[ "$EV_ACCEPTED" != "1" ]]; then
  echo "FAIL: expected events accepted=1, got ${EV_ACCEPTED}" >&2
  exit 1
fi
api GET /api/v1/admin/analytics/summary "" "$ADMIN_TOKEN"
AN_RETAINED="$(json_get "$HTTP_BODY" "obj.get('retained_events')")"
AN_USERS="$(json_get "$HTTP_BODY" "obj.get('distinct_users')")"
echo "analytics retained=${AN_RETAINED} distinct_users=${AN_USERS}"
if [[ -z "$AN_RETAINED" || "$AN_RETAINED" == "0" || "$AN_RETAINED" == "None" ]]; then
  echo "FAIL: expected analytics retained_events >= 1, got ${AN_RETAINED}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "Admin mute blocks chat + gifts; unmute restores chat"
# Plan 06 §8.3: 封禁/禁言策略生效 — control-plane assertion (in-memory moderation).
api POST /api/v1/admin/mute \
  "{\"user_id\":\"${FAN_ID}\",\"reason\":\"dogfood mute policy\"}" \
  "$ADMIN_TOKEN"
api_soft POST "/api/v1/rooms/${ROOM_ID}/messages" \
  "{\"body\":\"should be blocked while muted\"}" \
  "$FAN_TOKEN"
if [[ "$HTTP_STATUS" != "403" ]]; then
  echo "FAIL: muted fan chat should be 403, got HTTP ${HTTP_STATUS}" >&2
  echo "Body: ${HTTP_BODY}" >&2
  exit 1
fi
echo "muted chat blocked: HTTP ${HTTP_STATUS}"
if ! skip_mock; then
  api GET /api/v1/gifts
  MUTE_GIFT_ID="$(json_get "$HTTP_BODY" "sorted(obj['items'], key=lambda g: g.get('price', 0))[0]['id']")"
  api_soft POST "/api/v1/rooms/${ROOM_ID}/gifts" \
    "{\"gift_id\":\"${MUTE_GIFT_ID}\",\"receiver_id\":\"${HOST_ID}\",\"count\":1,\"client_request_id\":\"dogfood-mute-gift-$$\"}" \
    "$FAN_TOKEN"
  if [[ "$HTTP_STATUS" != "403" ]]; then
    echo "FAIL: muted fan gift should be 403, got HTTP ${HTTP_STATUS}" >&2
    echo "Body: ${HTTP_BODY}" >&2
    exit 1
  fi
  echo "muted gift blocked: HTTP ${HTTP_STATUS}"
else
  echo "SKIP muted gift assert (DOGFOOD_STRICT=1)"
fi
api POST /api/v1/admin/unmute \
  "{\"user_id\":\"${FAN_ID}\",\"reason\":\"dogfood unmute\"}" \
  "$ADMIN_TOKEN"
api POST "/api/v1/rooms/${ROOM_ID}/messages" \
  "{\"body\":\"ok after unmute\"}" \
  "$FAN_TOKEN"
MSG_AFTER_UNMUTE="$(json_get "$HTTP_BODY" "obj['id']")"
echo "after unmute message_id=${MSG_AFTER_UNMUTE}"

# ---------------------------------------------------------------------------
label "Admin ban blocks authed requests + re-login (dedicated user; no unban API)"
# Plan 06 §8.3: 封禁策略生效 — ban is sticky (no unban route in P1), so use a throwaway fan.
BAN_EMAIL="dogfood-ban-target-$(date +%s)-$$@example.com"
api POST /api/v1/auth/otp/send "{\"email\":\"${BAN_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${BAN_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
BAN_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
BAN_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "ban_target_id=${BAN_ID}"
# Sanity: banned-target can post before ban.
api POST "/api/v1/rooms/${ROOM_ID}/messages" \
  "{\"body\":\"pre-ban hello\"}" \
  "$BAN_TOKEN"
api POST /api/v1/admin/ban \
  "{\"user_id\":\"${BAN_ID}\",\"reason\":\"dogfood ban policy\"}" \
  "$ADMIN_TOKEN"
api_soft POST "/api/v1/rooms/${ROOM_ID}/messages" \
  "{\"body\":\"should be blocked while banned\"}" \
  "$BAN_TOKEN"
if [[ "$HTTP_STATUS" != "403" ]]; then
  echo "FAIL: banned user chat should be 403, got HTTP ${HTTP_STATUS}" >&2
  echo "Body: ${HTTP_BODY}" >&2
  exit 1
fi
if ! echo "$HTTP_BODY" | grep -qi 'banned\|FORBIDDEN'; then
  echo "WARN: ban response body unexpected: ${HTTP_BODY}" >&2
fi
echo "banned chat blocked: HTTP ${HTTP_STATUS}"
# Re-login must not mint a usable session for banned users.
api_soft POST /api/v1/auth/otp/send "{\"email\":\"${BAN_EMAIL}\"}"
api_soft POST /api/v1/auth/otp/verify "{\"email\":\"${BAN_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
if [[ "$HTTP_STATUS" =~ ^2 ]]; then
  echo "FAIL: OTP verify for banned user should not succeed (HTTP ${HTTP_STATUS})" >&2
  echo "Body: ${HTTP_BODY}" >&2
  exit 1
fi
echo "banned re-login blocked: HTTP ${HTTP_STATUS}"

# ---------------------------------------------------------------------------
label "Host stops room; GET room shows non-live"
api POST "/api/v1/rooms/${ROOM_ID}/stop" "" "$HOST_TOKEN"
ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "after stop: status=${ROOM_STATUS}"
api GET "/api/v1/rooms/${ROOM_ID}"
ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "get room: status=${ROOM_STATUS}"
if [[ "$ROOM_STATUS" == "live" ]]; then
  echo "FAIL: expected non-live status after stop, got ${ROOM_STATUS}" >&2
  exit 1
fi

echo
echo "DOGFOOD_API_SMOKE_PASS"
echo "room_id=${ROOM_ID} host=${HOST_ID} fan=${FAN_ID}"
echo "publish=${PUSH_URL}  stream_key=${STREAM_KEY}"
echo "hls=${HLS_URL}"
