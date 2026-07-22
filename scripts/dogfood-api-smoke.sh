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

HOST_EMAIL="dogfood-host-$(date +%s)@example.com"
FAN_EMAIL="dogfood-fan-$(date +%s)@example.com"

step=0
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

json_get() {
  # json_get <json> <python expr on obj, e.g. "obj['access_token']">
  local json="$1"
  local expr="$2"
  python3 -c "import json,sys; obj=json.loads(sys.argv[1]); print(${expr})" "$json"
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
api POST /api/v1/wallet/topups "{\"amount\":1000,\"reference\":\"dogfood-topup\"}" "$FAN_TOKEN"
BAL="$(json_get "$HTTP_BODY" "obj['balance']")"
echo "fan balance after topup=${BAL}"
api GET /api/v1/gifts
GIFT_ID="$(json_get "$HTTP_BODY" "obj['items'][0]['id']")"
GIFT_NAME="$(json_get "$HTTP_BODY" "obj['items'][0]['name']")"
echo "gift=${GIFT_NAME} id=${GIFT_ID}"
CLIENT_REQ="dogfood-gift-$(date +%s)-$$"
api POST "/api/v1/rooms/${ROOM_ID}/gifts" \
  "{\"gift_id\":\"${GIFT_ID}\",\"receiver_id\":\"${HOST_ID}\",\"count\":1,\"client_request_id\":\"${CLIENT_REQ}\"}" \
  "$FAN_TOKEN"
GIFT_ORDER="$(json_get "$HTTP_BODY" "obj['id']")"
REPLAYED="$(json_get "$HTTP_BODY" "obj['replayed']")"
echo "gift_order=${GIFT_ORDER} replayed=${REPLAYED}"

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
