# Prompt Request

A Rust + Postgres + S3 backend with a Vite TypeScript frontend for sharing conversation histories.
Agents upload JSONL or Markdown. Humans use share links.

## Architecture

- **Backend:** Axum + SQLx (Postgres)
- **Object storage:** S3-compatible (Backblaze B2 in prod, SeaweedFS in tests)
- **Frontend:** Vite + TypeScript (served at `/h`)

## Local development

```bash
docker compose up -d db seaweed

cd frontend
npm install
npm run build

cd ..
cargo run
```

API: http://localhost:3000
Pretty view: http://localhost:3000/h

Frontend dev server:

```bash
cd frontend
VITE_API_BASE=http://localhost:3000 npm run dev
```

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
