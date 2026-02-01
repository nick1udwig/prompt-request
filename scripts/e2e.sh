#!/usr/bin/env bash
set -euo pipefail

docker compose -f docker-compose.e2e.yml up -d db seaweed
trap "docker compose -f docker-compose.e2e.yml down" EXIT
sleep 2

wait_for_port() {
  local host=$1
  local port=$2
  local name=$3
  local retries=30
  local count=0
  until (echo >"/dev/tcp/${host}/${port}") >/dev/null 2>&1; do
    count=$((count + 1))
    if [ "$count" -ge "$retries" ]; then
      echo "Timed out waiting for ${name} on ${host}:${port}" >&2
      exit 1
    fi
    sleep 1
  done
}

wait_for_port localhost 5433 "postgres"
wait_for_port localhost 8334 "seaweedfs"
sleep 2

wait_for_http() {
  local url=$1
  local name=$2
  local retries=30
  local count=0
  while true; do
    local code
    code=$(curl -s -o /dev/null -w '%{http_code}' "$url" || true)
    if [ "$code" != "000" ]; then
      break
    fi
    count=$((count + 1))
    if [ "$count" -ge "$retries" ]; then
      echo "Timed out waiting for ${name} http response at ${url}" >&2
      exit 1
    fi
    sleep 1
  done
}

wait_for_http http://localhost:8334 "seaweedfs s3"
sleep 2

export DATABASE_URL=postgres://prompt:prompt@localhost:5433/prompt_request
export S3_ENDPOINT=http://localhost:8334
export S3_REGION=us-east-1
export S3_BUCKET=prompt-requests
export S3_ACCESS_KEY_ID=prompt
export S3_SECRET_ACCESS_KEY=prompt
export S3_FORCE_PATH_STYLE=true
export S3_CREATE_BUCKET=true
export E2E=1

cargo test -- --nocapture
