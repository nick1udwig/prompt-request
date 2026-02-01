#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${BACKUP_ENV_FILE:-}" ]]; then
  if [[ -f "$BACKUP_ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$BACKUP_ENV_FILE"
  else
    echo "BACKUP_ENV_FILE not found: $BACKUP_ENV_FILE" >&2
    exit 1
  fi
fi

: "${DATABASE_URL:?DATABASE_URL is required}"
: "${DB_BACKUP_S3_BUCKET:?DB_BACKUP_S3_BUCKET is required}"

DB_BACKUP_KEEP="${DB_BACKUP_KEEP:-8}"
DB_BACKUP_S3_PREFIX="${DB_BACKUP_S3_PREFIX:-db-backups/}"
DB_BACKUP_TMP_DIR="${DB_BACKUP_TMP_DIR:-/tmp/prompt-request-backups}"
DB_BACKUP_FILE_PREFIX="${DB_BACKUP_FILE_PREFIX:-prompt-request-db}"
PG_DUMP_BIN="${PG_DUMP_BIN:-pg_dump}"

case "$DB_BACKUP_KEEP" in
  ''|*[!0-9]*)
    echo "DB_BACKUP_KEEP must be an integer" >&2
    exit 1
    ;;
esac

if [[ "$DB_BACKUP_S3_PREFIX" != */ ]]; then
  DB_BACKUP_S3_PREFIX="${DB_BACKUP_S3_PREFIX}/"
fi

if ! command -v "$PG_DUMP_BIN" >/dev/null 2>&1; then
  echo "pg_dump not found: $PG_DUMP_BIN" >&2
  exit 1
fi

if ! command -v aws >/dev/null 2>&1; then
  echo "aws CLI not found (install awscli)" >&2
  exit 1
fi

mkdir -p "$DB_BACKUP_TMP_DIR"

timestamp=$(date -u +%Y%m%d-%H%M%S)
filename="${DB_BACKUP_FILE_PREFIX}-${timestamp}.sql.gz"
tmpfile="${DB_BACKUP_TMP_DIR}/${filename}"

$PG_DUMP_BIN --no-owner --no-privileges "$DATABASE_URL" | gzip > "$tmpfile"

key="${DB_BACKUP_S3_PREFIX}${filename}"

aws s3 cp "$tmpfile" "s3://${DB_BACKUP_S3_BUCKET}/${key}" --only-show-errors
aws s3api head-object --bucket "$DB_BACKUP_S3_BUCKET" --key "$key" >/dev/null

rm -f "$tmpfile"

mapfile -t keys < <(
  aws s3api list-objects-v2 \
    --bucket "$DB_BACKUP_S3_BUCKET" \
    --prefix "$DB_BACKUP_S3_PREFIX" \
    --query 'Contents[].Key' \
    --output text |
  tr '\t' '\n' |
  grep -E "^${DB_BACKUP_S3_PREFIX}${DB_BACKUP_FILE_PREFIX}-[0-9]{8}-[0-9]{6}\\.sql\\.gz$" |
  sort || true
)

count=${#keys[@]}
if (( count > DB_BACKUP_KEEP )); then
  delete_count=$((count - DB_BACKUP_KEEP))
  for ((i=0; i<delete_count; i++)); do
    aws s3api delete-object \
      --bucket "$DB_BACKUP_S3_BUCKET" \
      --key "${keys[$i]}" \
      >/dev/null
  done
fi
