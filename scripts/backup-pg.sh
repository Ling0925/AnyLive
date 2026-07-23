#!/usr/bin/env bash
# Postgres logical dump for AnyLive (P2 backup-restore · control plane).
# Default: dump the compose postgres on localhost:5432.
#
# Usage:
#   ./scripts/backup-pg.sh
#   ./scripts/backup-pg.sh /path/to/out.dump
#   DATABASE_URL=postgres://... ./scripts/backup-pg.sh
#
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPORTS="${DOGFOOD_REPORT_DIR:-$ROOT/reports}"
mkdir -p "$REPORTS"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${1:-$REPORTS/pg-backup-${STAMP}.dump}"

PGUSER="${POSTGRES_USER:-anylive}"
PGPASSWORD="${POSTGRES_PASSWORD:-anylive}"
PGHOST="${POSTGRES_HOST:-127.0.0.1}"
PGPORT="${POSTGRES_PORT:-5432}"
PGDATABASE="${POSTGRES_DB:-anylive}"

if [ -n "${DATABASE_URL:-}" ]; then
  # Prefer pg_dump URL form when provided
  export PGPASSWORD
  echo "==> pg_dump from DATABASE_URL → $OUT"
  if command -v pg_dump >/dev/null 2>&1; then
    pg_dump --format=custom --no-owner --no-acl --file="$OUT" "$DATABASE_URL"
  else
    echo "==> host pg_dump missing — using docker compose postgres"
    docker compose -f "$ROOT/deploy/docker-compose.yml" exec -T postgres \
      pg_dump -U "$PGUSER" -d "$PGDATABASE" --format=custom --no-owner --no-acl \
      >"$OUT"
  fi
else
  export PGPASSWORD
  if command -v pg_dump >/dev/null 2>&1; then
    echo "==> pg_dump ${PGUSER}@${PGHOST}:${PGPORT}/${PGDATABASE} → $OUT"
    pg_dump -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" \
      --format=custom --no-owner --no-acl --file="$OUT"
  else
    echo "==> host pg_dump missing — using docker compose postgres"
    docker compose -f "$ROOT/deploy/docker-compose.yml" exec -T postgres \
      pg_dump -U "$PGUSER" -d "$PGDATABASE" --format=custom --no-owner --no-acl \
      >"$OUT"
  fi
fi

ls -la "$OUT"
echo "OK dump=$OUT"
echo "Restore drill: ./scripts/restore-pg-drill.sh $OUT"
