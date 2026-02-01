CREATE TABLE accounts (
    id BIGSERIAL PRIMARY KEY,
    api_key_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE TABLE requests (
    uuid UUID PRIMARY KEY,
    account_id BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    latest_rev INT NOT NULL DEFAULT 0
);

CREATE TABLE request_revisions (
    id BIGSERIAL PRIMARY KEY,
    request_uuid UUID NOT NULL REFERENCES requests(uuid) ON DELETE CASCADE,
    rev_number INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    content_type TEXT NOT NULL,
    size_bytes INT NOT NULL,
    sha256 TEXT NOT NULL,
    object_key TEXT NOT NULL,
    UNIQUE (request_uuid, rev_number)
);

CREATE INDEX request_revisions_request_rev_idx
    ON request_revisions (request_uuid, rev_number);
