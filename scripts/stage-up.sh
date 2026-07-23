#!/usr/bin/env bash
# Bring up a stage-topology stack on the local machine (P2 M2.1).
# Control-plane only — does not claim cloud stage, ESP, or store listing.
#
# Usage:
#   ./scripts/stage-up.sh
#   SKIP_DOGFOOD_SMOKE=1 ./scripts/stage-up.sh
#   STAGE_LOCAL_ALLOW_DEV_OTP=1 ./scripts/stage-up.sh   # local rehearsal OTP
#
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ENV_FILE="${STAGE_ENV_FILE:-$ROOT/deploy/.env.stage.local}"
ENV_EXAMPLE="$ROOT/deploy/.env.stage.local.example"
COMPOSE=(
  docker compose
  -f deploy/docker-compose.yml
  -f deploy/docker-compose.stage.yml
  --env-file "$ENV_FILE"
)

mint_secret() {
  # 40-char url-safe-ish secret
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -base64 36 | tr -d '/+=' | head -c 40
  else
    # fallback: date+pid noise
    printf 's%st%sp%s' "$$" "$(date +%s)" "$RANDOM$RANDOM$RANDOM" | head -c 40
  fi
  echo
}

ensure_env_file() {
  if [ -f "$ENV_FILE" ]; then
    return 0
  fi
  if [ ! -f "$ENV_EXAMPLE" ]; then
    echo "missing $ENV_EXAMPLE" >&2
    exit 1
  fi
  echo "==> creating $ENV_FILE from example (secrets minted)"
  cp "$ENV_EXAMPLE" "$ENV_FILE"
  # Replace CHANGE_ME_* placeholders with minted secrets (macOS/BSD sed)
  local key val
  for key in \
    JWT_ACCESS_SECRET \
    JWT_REFRESH_SECRET \
    CENTRIFUGO_API_KEY \
    CENTRIFUGO_TOKEN_SECRET \
    SRS_PUBLISH_SECRET \
    SRS_WEBHOOK_SECRET
  do
    val="$(mint_secret)"
    # Only replace lines that still look like CHANGE_ME
    if grep -q "^${key}=CHANGE_ME" "$ENV_FILE" 2>/dev/null; then
      if sed --version >/dev/null 2>&1; then
        sed -i "s|^${key}=CHANGE_ME.*|${key}=${val}|" "$ENV_FILE"
      else
        sed -i '' "s|^${key}=CHANGE_ME.*|${key}=${val}|" "$ENV_FILE"
      fi
    fi
  done
  # Ensure access ≠ refresh
  local a r
  a="$(grep -E '^JWT_ACCESS_SECRET=' "$ENV_FILE" | head -1 | cut -d= -f2-)"
  r="$(grep -E '^JWT_REFRESH_SECRET=' "$ENV_FILE" | head -1 | cut -d= -f2-)"
  if [ -n "$a" ] && [ "$a" = "$r" ]; then
    val="$(mint_secret)"
    if sed --version >/dev/null 2>&1; then
      sed -i "s|^JWT_REFRESH_SECRET=.*|JWT_REFRESH_SECRET=${val}|" "$ENV_FILE"
    else
      sed -i '' "s|^JWT_REFRESH_SECRET=.*|JWT_REFRESH_SECRET=${val}|" "$ENV_FILE"
    fi
  fi
  echo "    wrote secrets into $ENV_FILE (gitignored)"
}

apply_local_otp_override() {
  if [ "${STAGE_LOCAL_ALLOW_DEV_OTP:-0}" != "1" ]; then
    return 0
  fi
  echo "==> STAGE_LOCAL_ALLOW_DEV_OTP=1 — local rehearsal OTP (NOT ESP)"
  if grep -q '^ALLOW_DEV_OTP=' "$ENV_FILE"; then
    if sed --version >/dev/null 2>&1; then
      sed -i 's|^ALLOW_DEV_OTP=.*|ALLOW_DEV_OTP=1|' "$ENV_FILE"
      sed -i 's|^OTP_NOTIFIER=.*|OTP_NOTIFIER=log|' "$ENV_FILE"
    else
      sed -i '' 's|^ALLOW_DEV_OTP=.*|ALLOW_DEV_OTP=1|' "$ENV_FILE"
      sed -i '' 's|^OTP_NOTIFIER=.*|OTP_NOTIFIER=log|' "$ENV_FILE"
    fi
  else
    printf '\nALLOW_DEV_OTP=1\nOTP_NOTIFIER=log\n' >>"$ENV_FILE"
  fi
}

ensure_env_file
apply_local_otp_override

echo "==> compose config (stage overlay)"
"${COMPOSE[@]}" config --quiet

echo "==> build api + admin"
"${COMPOSE[@]}" --profile app build api admin

echo "==> up stage stack"
"${COMPOSE[@]}" --profile app up -d

echo "==> wait API /health"
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

echo "==> wait admin"
for i in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8090/ >/dev/null 2>&1; then
    echo "admin up"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "admin timeout" >&2
    exit 1
  fi
  sleep 1
done

echo
echo "Stage-topology stack ready (local):"
echo "  API:        http://localhost:8088/health"
echo "  metrics:    http://localhost:8088/metrics"
echo "  admin:      http://localhost:8090/"
echo "  env file:   $ENV_FILE"
echo "  APP_ENV:    $(grep -E '^APP_ENV=' "$ENV_FILE" | head -1 | cut -d= -f2- || echo staging)"
echo "  FEATURE_PK/COHOST remain 0 (P3 experimental)"
echo
echo "Smoke (honest):"
echo "  # If ALLOW_DEV_OTP=1 (rehearsal):"
echo "  API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 ./scripts/dogfood-api-smoke.sh"
echo "  # Real OTP stage:"
echo "  DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=http://127.0.0.1:8088 ./scripts/dogfood-api-smoke.sh"
echo
echo "Backup drill: ./scripts/backup-pg.sh && ./scripts/restore-pg-drill.sh <dump>"
echo "Docs: docs/runbooks/go-live-stage.md · docs/product/p2-status.md"
echo
echo "NOTE: local stage-up ≠ cloud stage ≠ store internal. Solo: no sign-off gates."
echo

if [ "${SKIP_DOGFOOD_SMOKE:-0}" = "1" ]; then
  echo "==> SKIP_DOGFOOD_SMOKE=1"
  exit 0
fi

# Only auto-smoke when fixed OTP is on (otherwise needs real OTP_CODE).
if grep -qE '^ALLOW_DEV_OTP=1' "$ENV_FILE" 2>/dev/null; then
  echo "==> dogfood-api-smoke (rehearsal OTP; DOGFOOD_STRICT=1 if mock topup off)"
  mkdir -p "$ROOT/reports"
  STRICT_FLAG=0
  if ! grep -qE '^ALLOW_MOCK_TOPUP=1' "$ENV_FILE" 2>/dev/null; then
    STRICT_FLAG=1
    echo "    ALLOW_MOCK_TOPUP≠1 → DOGFOOD_STRICT=1 (skip mock topup/pay)"
  fi
  if DOGFOOD_REPORT_DIR="$ROOT/reports" \
      API_BASE=http://127.0.0.1:8088 OTP_CODE=123456 DOGFOOD_STRICT="$STRICT_FLAG" \
      "$ROOT/scripts/dogfood-api-smoke.sh"; then
    echo "dogfood-api-smoke: PASS"
  else
    echo "dogfood-api-smoke: FAIL (stack left running)" >&2
    exit 1
  fi
else
  echo "==> ALLOW_DEV_OTP≠1 — skip auto smoke; pass OTP_CODE manually with DOGFOOD_STRICT if needed"
fi
