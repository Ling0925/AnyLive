#!/usr/bin/env bash
# Seed a local Postgres admin_users row so the ops console can force-close / mute / etc.
# Use when bootstrap is closed (admin_users non-empty) and you need a known operator.
#
# Usage:
#   ./scripts/seed-admin-local.sh ops@example.com
#   API_BASE=http://localhost:8088 OTP_CODE=123456 ./scripts/seed-admin-local.sh ops@example.com
#
# Prerequisites: API on :8088 (or API_BASE), docker compose postgres (anylive-postgres-1).

set -euo pipefail

EMAIL="${1:-}"
if [[ -z "$EMAIL" ]]; then
  echo "usage: $0 <email>" >&2
  exit 2
fi

API_BASE="${API_BASE:-http://localhost:8088}"
OTP_CODE="${OTP_CODE:-123456}"
PG_CONTAINER="${DOGFOOD_PG_CONTAINER:-anylive-postgres-1}"
PG_USER="${POSTGRES_USER:-anylive}"
PG_DB="${POSTGRES_DB:-anylive}"

echo "seed-admin-local: email=${EMAIL} api=${API_BASE}"

curl -sS -o /dev/null -X POST "${API_BASE}/api/v1/auth/otp/send" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"${EMAIL}\"}"

VERIFY=$(curl -sS -X POST "${API_BASE}/api/v1/auth/otp/verify" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"${EMAIL}\",\"code\":\"${OTP_CODE}\"}")

USER_ID=$(python3 -c "import json,sys; print(json.load(sys.stdin)['user']['id'])" <<<"$VERIFY")
TOKEN=$(python3 -c "import json,sys; print(json.load(sys.stdin)['access_token'])" <<<"$VERIFY")
echo "user_id=${USER_ID}"

if ! command -v docker >/dev/null 2>&1; then
  echo "FAIL: docker not available; cannot insert admin_users" >&2
  exit 1
fi
if ! docker ps --format '{{.Names}}' | grep -qx "$PG_CONTAINER"; then
  echo "FAIL: postgres container ${PG_CONTAINER} not running" >&2
  exit 1
fi

# role column exists after migration 010; keep INSERT compatible either way.
docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d "$PG_DB" -v ON_ERROR_STOP=1 <<SQL
INSERT INTO admin_users (user_id)
VALUES ('${USER_ID}')
ON CONFLICT (user_id) DO NOTHING;
SQL

# Prefer role=admin when column present (010).
docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d "$PG_DB" <<'SQL' >/dev/null 2>&1 || true
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'admin_users' AND column_name = 'role'
  ) THEN
    EXECUTE format(
      'UPDATE admin_users SET role = %L WHERE user_id = %L::uuid',
      'admin',
      current_setting('seed.uid', true)
    );
  END IF;
END $$;
SQL
# Direct update when role exists (simpler path).
docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d "$PG_DB" -c \
  "UPDATE admin_users SET role = 'admin' WHERE user_id = '${USER_ID}'::uuid;" \
  >/dev/null 2>&1 || true

HTTP=$(curl -sS -o /tmp/seed-admin-audit.json -w '%{http_code}' \
  -H "Authorization: Bearer ${TOKEN}" \
  "${API_BASE}/api/v1/admin/audit")
if [[ "$HTTP" != "200" ]]; then
  echo "FAIL: admin audit still HTTP ${HTTP} after seed" >&2
  cat /tmp/seed-admin-audit.json >&2 || true
  exit 1
fi

echo "OK: ${EMAIL} is admin. Login admin-web with OTP ${OTP_CODE}."
echo "verify: GET /api/v1/admin/audit → HTTP ${HTTP}"
