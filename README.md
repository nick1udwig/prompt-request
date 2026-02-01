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

## Production setup (high level)

1) **Run the app**  
   - Docker (recommended): build and run the image with environment variables set for Postgres + S3.
   - Or run the binary directly and set env vars via systemd.

2) **Configure required services**  
   - Postgres 16+  
   - S3-compatible storage (Backblaze B2, AWS S3, etc.)

3) **Set required environment variables**  
   - `DATABASE_URL`, `S3_BUCKET`  
   - `S3_ENDPOINT`, `S3_REGION`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`  
   - `API_KEY_PEPPER` (strongly recommended)  
   - Optional: `FRONT_PAGE_PATH`, `FRONTEND_DIST`, `RUST_LOG`, `DB_MAX_CONNECTIONS`

4) **Reverse proxy + TLS**  
   - Terminate TLS at Caddy/Nginx/Cloudflare.
   - Pass `X-Forwarded-For` and lock down the origin to trusted proxies.
   - Disable caching for `/` and `/api`.

5) **Backups**  
   - Use `./scripts/backup_db.sh` with a separate S3 bucket (see `docs/ops.md`).
