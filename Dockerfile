FROM node:20-bookworm AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ .
RUN npm run build

FROM rust:1.77-bookworm AS backend-builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
COPY migrations ./migrations
COPY frontpage.md ./frontpage.md
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /app/target/release/prompt-request /usr/local/bin/prompt-request
COPY --from=backend-builder /app/migrations /app/migrations
COPY --from=backend-builder /app/frontpage.md /app/frontpage.md
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
ENV BIND_ADDR=0.0.0.0:3000
ENV FRONTEND_DIST=/app/frontend/dist
ENV FRONT_PAGE_PATH=/app/frontpage.md
EXPOSE 3000
CMD ["prompt-request"]
