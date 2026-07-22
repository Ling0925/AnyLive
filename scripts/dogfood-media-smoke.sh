#!/usr/bin/env bash
# Media-plane dogfood smoke: control-plane publish/play consistency + optional SRS.
#
# Prerequisites:
#   cargo run -p anylive-api   # default :8088, dev OTP = 123456
# Optional (SRS liveness only — does not require OBS):
#   docker compose -f deploy/docker-compose.yml up -d srs
#
# Usage:
#   ./scripts/dogfood-media-smoke.sh
#   API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 ./scripts/dogfood-media-smoke.sh
#   SRS_API_BASE=http://127.0.0.1:1985 ./scripts/dogfood-media-smoke.sh
#   SKIP_SRS=1 ./scripts/dogfood-media-smoke.sh   # skip optional SRS probe
#
# Requires: curl, python3.
# Pure helpers live in scripts/media_smoke_lib.py (unit-tested).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
API_BASE="${API_BASE:-http://localhost:8088}"
OTP_CODE="${OTP_CODE:-123456}"
SRS_API_BASE="${SRS_API_BASE:-http://127.0.0.1:1985}"
SKIP_SRS="${SKIP_SRS:-0}"
API_BASE="${API_BASE%/}"

HOST_EMAIL="dogfood-media-host-$(date +%s)@example.com"

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
  local json="$1"
  local expr="$2"
  python3 -c "import json,sys; obj=json.loads(sys.argv[1]); print(${expr})" "$json"
}

echo "AnyLive dogfood media smoke"
echo "API_BASE=${API_BASE}  OTP_CODE=${OTP_CODE}  SRS_API_BASE=${SRS_API_BASE}"

# ---------------------------------------------------------------------------
label "API /health"
api GET /health
echo "health ok: ${HTTP_BODY}"

# ---------------------------------------------------------------------------
label "Optional SRS HTTP API (:1985)"
if [[ "$SKIP_SRS" == "1" ]]; then
  echo "SKIP_SRS=1 — not probing SRS"
else
  SRS_PROBE_URL="$(
    PYTHONPATH="${ROOT}/scripts${PYTHONPATH:+:$PYTHONPATH}" python3 - <<PY
from media_smoke_lib import srs_http_ok_url
print(srs_http_ok_url("${SRS_API_BASE}"))
PY
  )"
  if curl -sS -o /tmp/anylive-srs-probe.json -w "%{http_code}" --connect-timeout 2 --max-time 5 \
      "$SRS_PROBE_URL" > /tmp/anylive-srs-probe.status 2>/tmp/anylive-srs-probe.err; then
    SRS_STATUS="$(cat /tmp/anylive-srs-probe.status)"
    if [[ "$SRS_STATUS" =~ ^2[0-9][0-9]$ ]]; then
      echo "SRS ok: GET ${SRS_PROBE_URL} -> HTTP ${SRS_STATUS}"
      head -c 200 /tmp/anylive-srs-probe.json 2>/dev/null || true
      echo
    else
      echo "WARN: SRS returned HTTP ${SRS_STATUS} at ${SRS_PROBE_URL} (media plane may be down)"
    fi
  else
    echo "WARN: SRS not reachable at ${SRS_PROBE_URL} (optional; control-plane checks continue)"
    cat /tmp/anylive-srs-probe.err 2>/dev/null || true
  fi
fi

# ---------------------------------------------------------------------------
label "Host OTP + create room + start live"
api POST /api/v1/auth/otp/send "{\"email\":\"${HOST_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${HOST_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
HOST_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
HOST_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "host_id=${HOST_ID}"

api POST /api/v1/rooms "{\"title\":\"Dogfood Media Room\"}" "$HOST_TOKEN"
ROOM_ID="$(json_get "$HTTP_BODY" "obj['id']")"
echo "room_id=${ROOM_ID}"
api POST "/api/v1/rooms/${ROOM_ID}/start" "" "$HOST_TOKEN"
ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
if [[ "$ROOM_STATUS" != "live" ]]; then
  echo "FAIL: expected room status live, got ${ROOM_STATUS}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
label "media/publish + media/play consistency (media_smoke_lib)"
api POST "/api/v1/rooms/${ROOM_ID}/media/publish" "" "$HOST_TOKEN"
PUBLISH_JSON="$HTTP_BODY"
api GET "/api/v1/rooms/${ROOM_ID}/media/play"
PLAY_JSON="$HTTP_BODY"

PYTHONPATH="${ROOT}/scripts${PYTHONPATH:+:$PYTHONPATH}" python3 - <<PY
import json
import sys

from media_smoke_lib import (
    assert_stream_key_matches_room,
    parse_play_response,
    parse_publish_response,
)

room_id = ${ROOM_ID@Q}
publish = json.loads(${PUBLISH_JSON@Q})
play = json.loads(${PLAY_JSON@Q})

info = parse_publish_response(publish)
assert_stream_key_matches_room(info["stream_key"], room_id)
hls = parse_play_response(play, room_id)

print(f"server={info['server']}")
print(f"stream_key={info['stream_key']}")
print(f"push_url={info['push_url']}")
print(f"hls={hls}")
print("MEDIA_CONSISTENCY_OK")
PY

# ---------------------------------------------------------------------------
label "Stop room"
api POST "/api/v1/rooms/${ROOM_ID}/stop" "" "$HOST_TOKEN"
ROOM_STATUS="$(json_get "$HTTP_BODY" "obj['status']")"
echo "after stop: status=${ROOM_STATUS}"

echo
echo "DOGFOOD_MEDIA_SMOKE_PASS"
echo "room_id=${ROOM_ID} host=${HOST_ID}"
echo "Note: OBS → SRS → HLS play is manual; see scripts/dogfood-media.md"
