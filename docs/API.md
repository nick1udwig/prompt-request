# Prompt Request API

## Auth

Use:

```
Authorization: Bearer <api_key>
```

API keys are returned once from `POST /api/accounts`.

## Rate limits

- Account creation: 1/hour per IP
- Authenticated API requests: 1/sec per account
- Public reads: 1/sec per IP

## Content types

Accepted:

- `text/markdown`
- `application/x-ndjson` (JSONL)

Max upload size: 1 MB

## Create account

```
POST /api/accounts
```

Response:

```json
{ "api_key": "prq_..." }
```

## Create request

```
POST /api/requests
Authorization: Bearer <api_key>
Content-Type: text/markdown
```

Body: raw file content.

Response:

```json
{
  "uuid": "...",
  "rev": 1,
  "content_type": "text/markdown",
  "size_bytes": 123,
  "sha256": "...",
  "created_at": "..."
}
```

## Update request (new revision)

```
PUT /api/requests/:uuid
Authorization: Bearer <api_key>
Content-Type: application/x-ndjson
```

Response:

```json
{
  "uuid": "...",
  "rev": 2,
  "content_type": "application/x-ndjson",
  "size_bytes": 456,
  "sha256": "...",
  "created_at": "..."
}
```

## List requests (account)

```
GET /api/requests?limit=50&offset=0
Authorization: Bearer <api_key>
```

Response:

```json
[
  {
    "uuid": "...",
    "created_at": "...",
    "updated_at": "...",
    "latest_rev": 2,
    "latest_content_type": "text/markdown"
  }
]
```

## List revisions

```
GET /api/requests/:uuid/revisions
Authorization: Bearer <api_key>
```

## Revision metadata

```
GET /api/requests/:uuid/revisions/:rev
Authorization: Bearer <api_key>
```

## Delete request or revision

```
DELETE /api/requests/:uuid
DELETE /api/requests/:uuid?rev=3
```

## Public views

- Raw: `GET /:uuid`
- Raw specific revision: `GET /:uuid?rev=2`
- Pretty: `GET /h/:uuid`
- Pretty specific revision: `GET /h/:uuid?rev=2`
- Front page markdown: `GET /`
- Front page HTML: `GET /h`
