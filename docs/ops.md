# Operations

## Cloudflare + Caddy

- Cloudflare in front of Caddy works fine. Ensure requests reach the app with the real client IP.
- Cloudflare sends `X-Forwarded-For` and `CF-Connecting-IP`. Caddy will pass headers through by default.
- For correct rate limits, lock down the origin to Cloudflare IPs or strip untrusted direct traffic.
- Disable caching for `/api` and `/` (front page) on Cloudflare.

## Environment secrets

Using `.env` on the VPS is OK if you treat it like a secret file:

- Store it outside the repo
- `chmod 600` and owned by root or the service user
- Load via systemd `EnvironmentFile=` or an explicit `source` step
- Avoid printing env vars in logs

## Database backups to S3

`./scripts/backup_db.sh` will:

- Run `pg_dump` and gzip it
- Upload to a separate S3 bucket
- Verify upload
- Keep the newest 8 backups and delete older ones

### Prereqs

- `pg_dump` (postgres client)
- `aws` CLI configured with access to the backup bucket

### Env vars

Required:

- `DATABASE_URL`
- `DB_BACKUP_S3_BUCKET`

Optional:

- `DB_BACKUP_S3_PREFIX` (default: `db-backups/`)
- `DB_BACKUP_KEEP` (default: `8`)
- `DB_BACKUP_TMP_DIR` (default: `/tmp/prompt-request-backups`)
- `DB_BACKUP_FILE_PREFIX` (default: `prompt-request-db`)
- `PG_DUMP_BIN` (default: `pg_dump`)
- `BACKUP_ENV_FILE` (path to env file to source)

### Example env file

```
DATABASE_URL=postgres://prompt:prompt@127.0.0.1:5432/prompt_request
DB_BACKUP_S3_BUCKET=prompt-request-db-backups
DB_BACKUP_S3_PREFIX=prod/
DB_BACKUP_KEEP=8
AWS_REGION=us-east-1
```

### Cron

Run hourly:

```
0 * * * * BACKUP_ENV_FILE=/etc/prompt-request/backup.env /root/git/prompt-request/scripts/backup_db.sh >> /var/log/prompt-request-backup.log 2>&1
```

Confirm the backup bucket has versioning disabled (optional) and the app bucket is separate from DB backups.
