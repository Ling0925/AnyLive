#!/usr/bin/env bash
# Seed a password-capable admin (or operator) for Wave A dogfood.
#
# Usage:
#   ./scripts/seed-password-admin.sh
#   ADMIN_EMAIL=ops@example.com ADMIN_USERNAME=ops ADMIN_PASSWORD='ChangeMe123!' ./scripts/seed-password-admin.sh
#
# Flow:
#   1. FEATURE_PUBLIC_REGISTER=1 must be set for first-boot OTP create (or users already exist).
#   2. OTP-login bootstrap admin (dev OTP 123456).
#   3. Provision password user via POST /admin/users (role=admin).
#   4. Print password login curl example.
#
# Prerequisites: API on :8088 (or API_BASE), ALLOW_DEV_OTP for local.

set -euo pipefail

API_BASE="${API_BASE:-http://localhost:8088}"
OTP_CODE="${OTP_CODE:-123456}"
BOOTSTRAP_EMAIL="${BOOTSTRAP_EMAIL:-seed-bootstrap@anylive.local}"
ADMIN_EMAIL="${ADMIN_EMAIL:-ops@example.com}"
ADMIN_USERNAME="${ADMIN_USERNAME:-ops}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-ChangeMe123!}"
ADMIN_DISPLAY="${ADMIN_DISPLAY:-Ops Admin}"

echo "seed-password-admin: api=${API_BASE}"
echo "  bootstrap_email=${BOOTSTRAP_EMAIL}"
echo "  target username=${ADMIN_USERNAME} email=${ADMIN_EMAIL}"

# Bootstrap session via OTP (requires public_register or existing user).
curl -sS -o /dev/null -X POST "${API_BASE}/api/v1/auth/otp/send" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"${BOOTSTRAP_EMAIL}\"}" || true

VERIFY=$(curl -sS -X POST "${API_BASE}/api/v1/auth/otp/verify" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"${BOOTSTRAP_EMAIL}\",\"code\":\"${OTP_CODE}\"}" || true)

if ! python3 -c "import json,sys; json.loads(sys.argv[1])['access_token']" "$VERIFY" 2>/dev/null; then
  echo "FAIL: OTP verify failed. For first boot set FEATURE_PUBLIC_REGISTER=1 (dev all_enabled tests already do)." >&2
  echo "body: $VERIFY" >&2
  exit 1
fi

BOOT_TOKEN=$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['access_token'])" "$VERIFY")
BOOT_UID=$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['user']['id'])" "$VERIFY")
echo "bootstrap user_id=${BOOT_UID}"

# Grant admin (bootstrap or existing).
GRANT_CODE=$(curl -sS -o /tmp/seed-pw-grant.json -w '%{http_code}' \
  -X POST "${API_BASE}/api/v1/admin/grant" \
  -H "Authorization: Bearer ${BOOT_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "{\"user_id\":\"${BOOT_UID}\"}")
echo "grant status=${GRANT_CODE}"

CREATE=$(curl -sS -X POST "${API_BASE}/api/v1/admin/users" \
  -H "Authorization: Bearer ${BOOT_TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "$(python3 - <<PY
import json
print(json.dumps({
  "display_name": "${ADMIN_DISPLAY}",
  "email": "${ADMIN_EMAIL}",
  "username": "${ADMIN_USERNAME}",
  "password": "${ADMIN_PASSWORD}",
  "must_change_password": False,
  "role": "admin",
}))
PY
)")

if ! python3 -c "import json,sys; json.loads(sys.argv[1])['user']['id']" "$CREATE" 2>/dev/null; then
  echo "FAIL: create user failed: $CREATE" >&2
  exit 1
fi

USER_ID=$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['user']['id'])" "$CREATE")
echo "created user_id=${USER_ID} username=${ADMIN_USERNAME}"

# Smoke password login
LOGIN=$(curl -sS -X POST "${API_BASE}/api/v1/auth/password/login" \
  -H 'Content-Type: application/json' \
  -d "{\"identifier\":\"${ADMIN_USERNAME}\",\"password\":\"${ADMIN_PASSWORD}\"}")
if ! python3 -c "import json,sys; json.loads(sys.argv[1])['access_token']" "$LOGIN" 2>/dev/null; then
  echo "FAIL: password login failed: $LOGIN" >&2
  exit 1
fi
echo "OK: password login works for ${ADMIN_USERNAME}"
echo
echo "Login example:"
echo "  curl -sS -X POST ${API_BASE}/api/v1/auth/password/login \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"identifier\":\"${ADMIN_USERNAME}\",\"password\":\"${ADMIN_PASSWORD}\"}'"
