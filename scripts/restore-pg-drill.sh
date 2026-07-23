#!/usr/bin/env bash
# Restore a pg_dump custom file into an isolated drill database (P2).
# Does NOT overwrite the live `anylive` DB by default.
#
# Usage:
#   ./scripts/restore-pg-drill.sh reports/pg-backup-….dump
#   RESTORE_DB=anylive_drill_manual ./scripts/restore-pg-drill.sh path.dump
#
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DUMP="${1:-}"
if [ -z "$DUMP" ] || [ ! -f "$DUMP" ]; then
  echo "usage: $0 <custom-format.dump>" >&2
  exit 2
fi

PGUSER="${POSTGRES_USER:-anylive}"
PGPASSWORD="${POSTGRES_PASSWORD:-anylive}"
PGHOST="${POSTGRES_HOST:-127.0.0.1}"
PGPORT="${POSTGRES_PORT:-5432}"
export PGPASSWORD

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
# Postgres folds unquoted identifiers to lowercase — keep drill DB names lowercase.
RESTORE_DB="${RESTORE_DB:-anylive_drill_${STAMP}}"
RESTORE_DB="$(printf '%s' "$RESTORE_DB" | tr '[:upper:]' '[:lower:]')"
REPORTS="${DOGFOOD_REPORT_DIR:-$ROOT/reports}"
mkdir -p "$REPORTS"
REPORT="$REPORTS/backup-restore-${STAMP}.md"

run_sql() {
  local sql="$1"
  if command -v psql >/dev/null 2>&1; then
    psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d postgres -v ON_ERROR_STOP=1 -c "$sql"
  else
    docker compose -f "$ROOT/deploy/docker-compose.yml" exec -T postgres \
      psql -U "$PGUSER" -d postgres -v ON_ERROR_STOP=1 -c "$sql"
  fi
}

run_restore() {
  if command -v pg_restore >/dev/null 2>&1; then
    pg_restore -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$RESTORE_DB" \
      --no-owner --no-acl --exit-on-error "$DUMP"
  else
    # Stream dump into container pg_restore
    docker compose -f "$ROOT/deploy/docker-compose.yml" exec -T postgres \
      pg_restore -U "$PGUSER" -d "$RESTORE_DB" --no-owner --no-acl --exit-on-error \
      <"$DUMP"
  fi
}

START_TS="$(date +%s)"
echo "==> create isolated DB $RESTORE_DB"
run_sql "DROP DATABASE IF EXISTS ${RESTORE_DB};"
run_sql "CREATE DATABASE ${RESTORE_DB} OWNER ${PGUSER};"

echo "==> restore $DUMP → $RESTORE_DB"
run_restore
END_TS="$(date +%s)"
ELAPSED=$((END_TS - START_TS))
# Portable minutes (avoid gawk/mawk printf quirks in subshells)
if [ "$ELAPSED" -le 0 ]; then
  RTO_MIN="0.00"
else
  RTO_MIN="$(python3 -c "print(f'{$ELAPSED/60:.2f}')" 2>/dev/null || echo "$ELAPSED")"
fi

echo "==> sample checks"
MIG_COUNT=""
if command -v psql >/dev/null 2>&1; then
  MIG_COUNT="$(psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$RESTORE_DB" -tAc \
    "SELECT count(*) FROM _sqlx_migrations" 2>/dev/null || echo "n/a")"
else
  MIG_COUNT="$(docker compose -f "$ROOT/deploy/docker-compose.yml" exec -T postgres \
    psql -U "$PGUSER" -d "$RESTORE_DB" -tAc "SELECT count(*) FROM _sqlx_migrations" 2>/dev/null || echo "n/a")"
fi
MIG_COUNT="$(echo "$MIG_COUNT" | tr -d '[:space:]')"

{
  echo "# Backup restore drill — ${STAMP}"
  echo
  echo "- Environment: local-compose-drill"
  echo "- Backup source: \`${DUMP}\`"
  echo "- Restore DB: \`${RESTORE_DB}\` (isolated; live \`anylive\` untouched)"
  echo "- RTO (minutes): ${RTO_MIN} (elapsed_s=${ELAPSED})"
  echo "- Migration rows (_sqlx_migrations): ${MIG_COUNT}"
  echo "- Reconcile: run against a live API pointed at this DB if needed"
  echo "  (\`GET /api/v1/admin/wallet/reconcile\`)"
  echo "- Issues: (none recorded by script)"
  echo "- Sign-off: solo owner self-test (no ceremony)"
  echo
  echo "Cleanup:"
  echo "\`\`\`sql"
  echo "DROP DATABASE IF EXISTS ${RESTORE_DB};"
  echo "\`\`\`"
} | tee "$REPORT"

echo "OK report=$REPORT elapsed_s=$ELAPSED"
