#!/usr/bin/env bash
# Build and start AnyLive test stack: deps + API + admin-web.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test)

echo "==> docker compose config"
"${COMPOSE[@]}" config --quiet

echo "==> build api + admin (this may take several minutes on first run)"
"${COMPOSE[@]}" --profile app build api admin

echo "==> up full test profile"
"${COMPOSE[@]}" --profile app up -d

echo "==> wait for API health"
for i in $(seq 1 60); do
  if curl -fsS http://127.0.0.1:8088/health >/dev/null 2>&1; then
    echo "API healthy"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "API health timeout" >&2
    "${COMPOSE[@]}" --profile app logs --tail=80 api || true
    exit 1
  fi
  sleep 2
done

echo "==> wait for admin"
for i in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8090/ >/dev/null 2>&1; then
    echo "admin reachable"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "admin timeout" >&2
    "${COMPOSE[@]}" --profile app logs --tail=40 admin || true
    exit 1
  fi
  sleep 1
done

echo
echo "Test stack is up:"
echo "  API:   http://localhost:8088/health"
echo "  Admin: http://localhost:8090/"
echo "  SRS HLS base: http://localhost:8080/live"
echo "  Centrifugo:   http://localhost:8001"
echo
echo "Dev OTP code: 123456  (ALLOW_DEV_OTP=1 / APP_ENV=local)"
echo
echo "OBS go-live (after host media/publish):"
echo "  Service:    Custom..."
echo "  Server:     rtmp://localhost:1935/live"
echo "  Stream key: signed token from POST /api/v1/rooms/{id}/media/publish"
echo "              (format room_id_exp_sig — not bare room UUID)"
echo "  Watch HLS:  GET /api/v1/rooms/{id}/media/play  or H5 ?room={id}"
echo "  Full guide: docs/runbooks/go-live-local.md"
echo
echo "Stop with: docker compose -f deploy/docker-compose.yml --profile app down"
echo

# Optional dogfood smokes against the stack we just brought up.
# SKIP_DOGFOOD_SMOKE=1 to skip; failures are non-fatal so the stack stays up.
if [ "${SKIP_DOGFOOD_SMOKE:-0}" = "1" ]; then
  echo "==> SKIP_DOGFOOD_SMOKE=1 — not running dogfood smokes"
else
  echo "==> dogfood API smoke"
  if API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 \
      "$ROOT/scripts/dogfood-api-smoke.sh"; then
    echo "dogfood-api-smoke: OK"
  else
    echo "WARN: dogfood-api-smoke failed (stack is still up; re-run: ./scripts/dogfood-api-smoke.sh)" >&2
  fi

  echo
  echo "==> dogfood media smoke"
  if API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 SRS_API_BASE=http://127.0.0.1:1985 \
      "$ROOT/scripts/dogfood-media-smoke.sh"; then
    echo "dogfood-media-smoke: OK"
  else
    echo "WARN: dogfood-media-smoke failed (stack is still up; re-run: ./scripts/dogfood-media-smoke.sh)" >&2
  fi
fi
