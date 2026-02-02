# Prompt Request

A Rust + Postgres + S3 backend with a Vite TypeScript frontend for sharing conversation histories.
Agents upload JSONL or Markdown. Humans use share links.
Support open source by sponsoring https://github.com/sponsors/nick1udwig

## Architecture

- **Backend:** Axum + SQLx (Postgres)
- **Object storage:** S3-compatible (Backblaze B2 in prod, SeaweedFS in tests)
- **Frontend:** Vite + TypeScript (served at `/h`)

## Local development

Fastest way to get everything running:

```bash
docker compose up --build app
```

API + UI: http://localhost:3000  
Raw markdown: http://localhost:3000/  
Pretty view: http://localhost:3000/h

If you want to hack on the frontend with live reload:

```bash
cd frontend
VITE_API_BASE=http://localhost:3000 npm run dev
```

Keep the backend running separately (via `cargo run` or `docker compose up app`). The dev server only serves the UI.

## Environment variables

Required:

- `DATABASE_URL`
- `S3_BUCKET`

Recommended:

- `S3_ENDPOINT` (B2 or SeaweedFS S3 endpoint)
- `S3_REGION` (default: `us-east-1`)
- `S3_ACCESS_KEY_ID`
- `S3_SECRET_ACCESS_KEY`
- `S3_FORCE_PATH_STYLE` (default: `true`)
- `S3_CREATE_BUCKET` (default: `true`)
- `BIND_ADDR` (default: `0.0.0.0:3000`)
- `API_KEY_PEPPER` (optional secret pepper for API key hashing)
- `FRONTEND_DIST` (default: `frontend/dist`)
- `FRONT_PAGE_PATH` (optional override for front page markdown)

## API summary

- `POST /api/accounts`
- `POST /api/requests`
- `PUT /api/requests/:uuid`
- `DELETE /api/requests/:uuid` (optional `?rev=`)
- `GET /api/requests`
- `GET /api/requests/:uuid/revisions`
- `GET /api/requests/:uuid/revisions/:rev`

Public:

- `GET /` (front page markdown)
- `GET /:uuid` (raw)
- `GET /h` (pretty front page)
- `GET /h/:uuid` (pretty view)

See `docs/API.md` for detailed examples.

## Model usage (agents)

- Before creating an account, check for `~/.prompt-request/.api-key`. If it exists, reuse that key.
- If missing, call `POST /api/accounts` and store the returned key in `~/.prompt-request/.api-key` for future use.

## Testing

Unit tests:

```bash
cargo test
```

End-to-end (SeaweedFS + Postgres):

```bash
./scripts/e2e.sh
```

The script uses `docker-compose.e2e.yml` to avoid port collisions.

## Notes

- Rate limiting is in-memory (single-instance only).
- Metadata fields are not stored yet; add a JSONB column later if needed.

## Operations

See `docs/ops.md` for Cloudflare/Caddy notes and the hourly DB backup cron job.

## Production setup (commands)

Prereqs: a Postgres 16+ database and an S3-compatible bucket.

### Option A: Docker (single host)

Build the image:

```bash
docker build -t prompt-request .
```

Run it (replace the values):

```bash
docker run -d --name prompt-request \
  -p 3000:3000 \
  -e DATABASE_URL="postgres://USER:PASS@HOST:5432/DBNAME" \
  -e S3_BUCKET="your-bucket" \
  -e S3_ENDPOINT="https://s3.us-east-1.amazonaws.com" \
  -e S3_REGION="us-east-1" \
  -e S3_ACCESS_KEY_ID="AKIA..." \
  -e S3_SECRET_ACCESS_KEY="SECRET..." \
  -e API_KEY_PEPPER="$(openssl rand -hex 32)" \
  -e RUST_LOG="info" \
  prompt-request
```

### Option B: Systemd (binary on host)

Build and install:

```bash
cargo build --release
sudo install -m 0755 target/release/prompt-request /usr/local/bin/prompt-request
```

Create an env file:

```bash
sudo mkdir -p /etc/prompt-request
sudo tee /etc/prompt-request/env >/dev/null <<'EOF'
DATABASE_URL=postgres://USER:PASS@HOST:5432/DBNAME
S3_BUCKET=your-bucket
S3_ENDPOINT=https://s3.us-east-1.amazonaws.com
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=AKIA...
S3_SECRET_ACCESS_KEY=SECRET...
API_KEY_PEPPER=$(openssl rand -hex 32)
RUST_LOG=info
EOF
```

Create a systemd unit:

```bash
sudo tee /etc/systemd/system/prompt-request.service >/dev/null <<'EOF'
[Unit]
Description=Prompt Request
After=network.target

[Service]
EnvironmentFile=/etc/prompt-request/env
ExecStart=/usr/local/bin/prompt-request
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now prompt-request
```

### Reverse proxy + TLS

- Terminate TLS at Caddy/Nginx/Cloudflare.
- Pass `X-Forwarded-For` and lock down the origin to trusted proxies.
- Disable caching for `/` and `/api`.

### Backups

- Use `./scripts/backup_db.sh` with a separate S3 bucket (see `docs/ops.md`).

## Migrating to a new VPS (Docker)

Your data lives in Postgres and S3-compatible object storage. If you use an external S3
provider (AWS/B2/etc.), you only need to move the Postgres database. If you use the
local SeaweedFS container, move both Postgres and the Seaweed volume.

### 1) On the old VPS (stop writes, back up)

```bash
docker compose stop app

# Postgres backup
docker compose exec -T db pg_dump -U prompt prompt_request | gzip > db.sql.gz

# SeaweedFS backup (skip if using external S3)
docker run --rm \
  -v prompt-request_seaweed_data:/data \
  -v "$PWD":/backup \
  alpine sh -c "tar czf /backup/seaweed_data.tgz -C /data ."
```

### 2) Copy backups to the new VPS

```bash
scp db.sql.gz seaweed_data.tgz user@NEW_HOST:/path/to/backups/
```

### 3) On the new VPS (restore)

```bash
docker compose up -d db seaweed

# Restore Postgres
gunzip -c /path/to/backups/db.sql.gz | docker compose exec -T db psql -U prompt -d prompt_request

# Restore SeaweedFS volume (skip if using external S3)
docker compose stop seaweed
docker run --rm \
  -v prompt-request_seaweed_data:/data \
  -v /path/to/backups:/backup \
  alpine sh -c "rm -rf /data/* && tar xzf /backup/seaweed_data.tgz -C /data"
docker compose up -d seaweed
```

### 4) Start the app

```bash
docker compose up -d app
```
