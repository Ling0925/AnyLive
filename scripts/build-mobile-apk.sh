#!/usr/bin/env bash
# Build a Flutter debug or release APK for operator self-test / internal dogfood.
#
# Operator-facing only — does NOT upload to Play Store / TestFlight.
# See apps/mobile/store/README.md and docs/runbooks/store-internal.md for store paths.
#
# Usage (from repo root):
#   ./scripts/build-mobile-apk.sh
#   ./scripts/build-mobile-apk.sh debug
#   ./scripts/build-mobile-apk.sh release
#   APP_FLAVOR=stage API_BASE_URL=https://api.stage.example.com ./scripts/build-mobile-apk.sh release
#   APP_FLAVOR=local API_BASE_URL=http://10.0.2.2:8088 ./scripts/build-mobile-apk.sh debug
#
# Env:
#   APP_FLAVOR      local|stage|prod (default: local)
#   API_BASE_URL    dart-define (default: http://10.0.2.2:8088 for Android emu-friendly local)
#   APP_ENV         optional; defaults to APP_FLAVOR
#   CENTRIFUGO_WS   optional WS URL dart-define
#   EMBEDDED_PLAYER true|false — ANYLIVE_EMBEDDED_PLAYER (default: false)
#   MODE            debug|release (default: first arg or debug)
#   OUT_DIR         copy APK here (default: reports/apk)
#
# Output:
#   Flutter default path under apps/mobile/build/app/outputs/flutter-apk/
#   Plus a dated copy under reports/apk/ when OUT_DIR is writable.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOBILE="$ROOT/apps/mobile"

MODE="${1:-${MODE:-debug}}"
case "$MODE" in
  debug|release) ;;
  *)
    echo "usage: $0 [debug|release]" >&2
    exit 2
    ;;
esac

APP_FLAVOR="${APP_FLAVOR:-local}"
# Default API base favors Android emulator loopback to host. Override for real device LAN IP.
API_BASE_URL="${API_BASE_URL:-http://10.0.2.2:8088}"
APP_ENV="${APP_ENV:-$APP_FLAVOR}"
CENTRIFUGO_WS="${CENTRIFUGO_WS:-}"
EMBEDDED_PLAYER="${EMBEDDED_PLAYER:-false}"
OUT_DIR="${OUT_DIR:-$ROOT/reports/apk}"

if [[ ! -d "$MOBILE" ]]; then
  echo "FAIL: missing $MOBILE" >&2
  exit 1
fi

if ! command -v flutter >/dev/null 2>&1; then
  echo "FAIL: flutter not on PATH" >&2
  exit 1
fi

DEFINES=(
  "--dart-define=APP_FLAVOR=${APP_FLAVOR}"
  "--dart-define=APP_ENV=${APP_ENV}"
  "--dart-define=API_BASE_URL=${API_BASE_URL}"
  "--dart-define=ANYLIVE_EMBEDDED_PLAYER=${EMBEDDED_PLAYER}"
)
if [[ -n "$CENTRIFUGO_WS" ]]; then
  DEFINES+=("--dart-define=CENTRIFUGO_WS=${CENTRIFUGO_WS}")
fi

echo "==> build-mobile-apk: mode=${MODE} flavor=${APP_FLAVOR} api=${API_BASE_URL}"
cd "$MOBILE"

flutter pub get

if [[ "$MODE" == "debug" ]]; then
  flutter build apk --debug "${DEFINES[@]}"
  APK_SRC="$MOBILE/build/app/outputs/flutter-apk/app-debug.apk"
else
  flutter build apk --release "${DEFINES[@]}"
  APK_SRC="$MOBILE/build/app/outputs/flutter-apk/app-release.apk"
fi

if [[ ! -f "$APK_SRC" ]]; then
  echo "FAIL: expected APK missing: $APK_SRC" >&2
  exit 1
fi

STAMP="$(date +%Y%m%dT%H%M%S)"
SAFE_FLAVOR="$(printf '%s' "$APP_FLAVOR" | tr -cd 'a-zA-Z0-9._-')"
APK_NAME="anylive-${SAFE_FLAVOR}-${MODE}-${STAMP}.apk"

mkdir -p "$OUT_DIR"
APK_DST="$OUT_DIR/$APK_NAME"
cp -f "$APK_SRC" "$APK_DST"

echo
echo "APK (flutter): $APK_SRC"
echo "APK (copy):    $APK_DST"
echo "Install example:"
echo "  adb install -r \"$APK_DST\""
echo "  # real device + host API: adb reverse tcp:8088 tcp:8088"
echo "Operator only — no store upload from this script."
