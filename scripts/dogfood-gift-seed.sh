#!/usr/bin/env bash
# Upsert a stable dogfood gift catalog via admin API.
#
# Seeds (or re-upserts by fixed id):
#   Rose / 1, Heart / 10, Rocket / 100
# Prints public + admin catalog after upsert.
#
# Prerequisites:
#   API on API_BASE (default http://localhost:8088), curl, python3.
#   Admin path: first-boot grant, DOGFOOD_ADMIN_EMAIL, or docker postgres seed.
#
# Usage:
#   ./scripts/dogfood-gift-seed.sh
#   API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 ./scripts/dogfood-gift-seed.sh
#   DOGFOOD_ADMIN_EMAIL=ops@example.com ./scripts/dogfood-gift-seed.sh
#
# Env (shared with dogfood-api-smoke.sh):
#   API_BASE, OTP_CODE, DOGFOOD_ADMIN_EMAIL, DOGFOOD_PG_CONTAINER,
#   POSTGRES_USER, POSTGRES_DB

set -euo pipefail

export OTP_CODE="${OTP_CODE:-123456}"
API_BASE="${API_BASE:-http://localhost:8088}"
API_BASE="${API_BASE%/}"
OTP_CODE="${OTP_CODE:-123456}"

# Stable UUIDs so re-runs upsert instead of creating duplicates.
GIFT_ROSE_ID="${DOGFOOD_GIFT_ROSE_ID:-a1000000-0000-4000-8000-000000000001}"
GIFT_HEART_ID="${DOGFOOD_GIFT_HEART_ID:-a1000000-0000-4000-8000-000000000002}"
GIFT_ROCKET_ID="${DOGFOOD_GIFT_ROCKET_ID:-a1000000-0000-4000-8000-000000000003}"

SEED_EMAIL="${DOGFOOD_GIFT_SEED_EMAIL:-dogfood-gift-ops-$(date +%s)@example.com}"

step=0

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

# Ensure ADMIN_TOKEN can call admin routes (same strategy as dogfood-api-smoke).
ensure_admin() {
  local seed_user_id="$1"
  local seed_token="$2"
  ADMIN_TOKEN=""

  api_soft POST /api/v1/admin/grant "{\"user_id\":\"${seed_user_id}\"}" "$seed_token"
  if [[ "$HTTP_STATUS" =~ ^2[0-9][0-9]$ ]]; then
    echo "admin bootstrap grant: ok (HTTP ${HTTP_STATUS})"
    ADMIN_TOKEN="$seed_token"
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
    echo "seeding admin_users for seed user via docker exec ${pg_container}"
    docker exec -i "$pg_container" \
      psql -U "$pg_user" -d "$pg_db" -v ON_ERROR_STOP=1 \
      -c "INSERT INTO admin_users (user_id) VALUES ('${seed_user_id}') ON CONFLICT DO NOTHING;" \
      >/dev/null
    docker exec -i "$pg_container" psql -U "$pg_user" -d "$pg_db" -c \
      "UPDATE admin_users SET role = 'admin' WHERE user_id = '${seed_user_id}'::uuid;" \
      >/dev/null 2>&1 || true
    ADMIN_TOKEN="$seed_token"
    api_soft GET /api/v1/admin/gifts "" "$ADMIN_TOKEN"
    if [[ "$HTTP_STATUS" =~ ^2[0-9][0-9]$ ]]; then
      echo "admin seed verified (admin gifts HTTP ${HTTP_STATUS})"
      return 0
    fi
    echo "WARN: admin seed did not unlock gifts (HTTP ${HTTP_STATUS}: ${HTTP_BODY})" >&2
  fi

  echo "FAIL: no admin path available." >&2
  echo "  - empty DB bootstrap grant failed (admin already exists), and" >&2
  echo "  - DOGFOOD_ADMIN_EMAIL not set, and" >&2
  echo "  - docker postgres seed unavailable/failed." >&2
  echo "Set DOGFOOD_ADMIN_EMAIL to an existing admin, or run:" >&2
  echo "  ./scripts/seed-admin-local.sh ops@example.com" >&2
  exit 1
}

upsert_gift() {
  local id="$1"
  local name="$2"
  local price="$3"
  api POST /api/v1/admin/gifts \
    "{\"id\":\"${id}\",\"name\":\"${name}\",\"price\":${price},\"active\":true}" \
    "$ADMIN_TOKEN"
  local out_id out_name out_price
  out_id="$(json_get "$HTTP_BODY" "obj['id']")"
  out_name="$(json_get "$HTTP_BODY" "obj['name']")"
  out_price="$(json_get "$HTTP_BODY" "obj['price']")"
  echo "upserted gift id=${out_id} name=${out_name} price=${out_price}"
}

echo "AnyLive dogfood gift catalog seed"
echo "API_BASE=${API_BASE}  OTP_CODE=${OTP_CODE}"

label "OTP login seed operator (${SEED_EMAIL})"
api POST /api/v1/auth/otp/send "{\"email\":\"${SEED_EMAIL}\"}"
api POST /api/v1/auth/otp/verify "{\"email\":\"${SEED_EMAIL}\",\"code\":\"${OTP_CODE}\"}"
SEED_TOKEN="$(json_get "$HTTP_BODY" "obj['access_token']")"
SEED_ID="$(json_get "$HTTP_BODY" "obj['user']['id']")"
echo "seed_user_id=${SEED_ID}"

label "Ensure admin"
ensure_admin "$SEED_ID" "$SEED_TOKEN"

label "Upsert Rose / Heart / Rocket"
upsert_gift "$GIFT_ROSE_ID" "Rose" 1
upsert_gift "$GIFT_HEART_ID" "Heart" 10
upsert_gift "$GIFT_ROCKET_ID" "Rocket" 100

label "Admin gift catalog"
api GET /api/v1/admin/gifts "" "$ADMIN_TOKEN"
python3 -c "
import json, sys
obj = json.loads(sys.argv[1])
items = obj.get('items') or []
print(f'admin_gift_count={len(items)}')
for g in items:
    print(f\"  {g.get('id')}  {g.get('name')}  price={g.get('price')}  active={g.get('active')}\")
" "$HTTP_BODY"

label "Public gift catalog"
api GET /api/v1/gifts
python3 -c "
import json, sys
obj = json.loads(sys.argv[1])
items = obj.get('items') or []
print(f'public_gift_count={len(items)}')
for g in items:
    print(f\"  {g.get('id')}  {g.get('name')}  price={g.get('price')}  active={g.get('active')}\")
needed = {'Rose', 'Heart', 'Rocket'}
names = {g.get('name') for g in items}
missing = needed - names
if missing:
    print('FAIL: public catalog missing: ' + ', '.join(sorted(missing)), file=sys.stderr)
    sys.exit(1)
" "$HTTP_BODY"

echo
echo "DOGFOOD_GIFT_SEED_PASS"
echo "rose_id=${GIFT_ROSE_ID} heart_id=${GIFT_HEART_ID} rocket_id=${GIFT_ROCKET_ID}"
