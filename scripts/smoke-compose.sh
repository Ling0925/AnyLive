#!/usr/bin/env bash
# Smoke-check that compose file is valid and critical config files exist.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

test -f deploy/docker-compose.yml
test -f deploy/centrifugo/config.json
test -f deploy/srs/srs.conf

# Ensure local SRS conf points hooks at the control-plane API.
python3 - <<'PY'
from pathlib import Path
text = Path("deploy/srs/srs.conf").read_text()
assert "http_hooks" in text, "srs.conf missing http_hooks"
assert "on_publish" in text and "on_unpublish" in text
assert "/api/v1/webhooks/srs/on_publish" in text
print("OK: srs.conf http_hooks point at API")
PY

if command -v docker >/dev/null 2>&1; then
  docker compose -f deploy/docker-compose.yml config --quiet
  echo "OK: docker compose config valid"
else
  echo "WARN: docker not available; skipped compose config"
fi

# Validate centrifugo JSON
python3 -c 'import json; json.load(open("deploy/centrifugo/config.json"))'
echo "OK: centrifugo config JSON valid"
echo "SMOKE_PASS"
