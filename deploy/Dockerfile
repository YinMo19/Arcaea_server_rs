# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.95-bookworm
ARG NODE_VERSION=24-bookworm-slim
ARG PNPM_VERSION=10.33.2

FROM node:${NODE_VERSION} AS frontend-builder
ARG PNPM_VERSION
WORKDIR /src/frontend
RUN corepack enable && corepack prepare "pnpm@${PNPM_VERSION}" --activate
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

FROM rust:${RUST_VERSION} AS rust-builder
WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock build.rs ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src

ENV SQLX_OFFLINE=true
RUN cargo build --release --locked \
    --bin Arcaea_server_rs \
    --bin init_db \
    --bin linkplayd \
    --bin sync_s3_manifest

FROM debian:bookworm-slim AS app
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl netcat-openbsd tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system arcaea \
    && useradd --system --gid arcaea --home-dir /app --shell /usr/sbin/nologin arcaea

COPY --from=rust-builder /src/target/release/Arcaea_server_rs /usr/local/bin/Arcaea_server_rs
COPY --from=rust-builder /src/target/release/init_db /usr/local/bin/init_db
COPY --from=rust-builder /src/target/release/linkplayd /usr/local/bin/linkplayd
COPY --from=rust-builder /src/target/release/sync_s3_manifest /usr/local/bin/sync_s3_manifest
COPY --from=frontend-builder /src/frontend/dist /app/web
COPY assets /app/assets
COPY deploy/docker-entrypoint.sh /usr/local/bin/docker-entrypoint

RUN mkdir -p /app/songs /app/bundles \
    && chmod +x /usr/local/bin/docker-entrypoint \
    && chown -R arcaea:arcaea /app

USER arcaea

ENV RUST_LOG=info \
    ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8090 \
    ASSET_DIR=/app/assets \
    ARC_DATA_FILE=/app/assets/arc_data.json \
    STORAGE_BACKEND=local

EXPOSE 8090
ENTRYPOINT ["tini", "--", "docker-entrypoint"]
CMD ["server"]

FROM caddy:2.9-alpine AS caddy
COPY deploy/Caddyfile /etc/caddy/Caddyfile
COPY --from=frontend-builder /src/frontend/dist /srv/web
EXPOSE 80 443 443/udp

FROM caddy:2.9-builder-alpine AS caddy-cloudflare-builder
RUN xcaddy build --with github.com/caddy-dns/cloudflare

FROM caddy:2.9-alpine AS caddy-cloudflare
COPY --from=caddy-cloudflare-builder /usr/bin/caddy /usr/bin/caddy
COPY deploy/Caddyfile /etc/caddy/Caddyfile
COPY --from=frontend-builder /src/frontend/dist /srv/web
EXPOSE 80 443 443/udp
