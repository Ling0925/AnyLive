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
echo "Dev OTP code: 123456  (APP_ENV=local)"
echo "Stop with: docker compose -f deploy/docker-compose.yml --profile app down"
