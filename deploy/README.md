# Docker Compose Deployment

This directory contains a quick deployment stack for the Rust server, admin frontend, MariaDB, Redis, Caddy, Link Play, and optional MinIO.

## Local File Storage

```sh
cd deploy
cp .env.example .env
docker compose up -d --build
```

Open `http://127.0.0.1:8080/web/` for the admin frontend. The default admin account is `admin / admin` unless changed in `deploy/.env`.

The compose file bind-mounts these paths from the repository root:

- `../assets` -> `/app/assets`
- `../songs` -> `/app/songs`
- `../bundles` -> `/app/bundles`

Put song files and `songlist` under `songs/`, and content bundles under `bundles/`, before starting the stack if you want local downloads to work.

The Rust server embeds SQL migrations into the `Arcaea_server_rs` binary at build time through `sqlx::migrate!("./migrations")`. Runtime containers and systemd deployment directories do not need a `migrations/` directory; pending migrations run when the server starts. The source `migrations/` directory is still required while building the binary.

## Caddy

Caddy is the public HTTP entrypoint:

- `/web/*` serves the built React admin frontend.
- `/web/api/*` proxies to the Rust backend unchanged.
- all other paths proxy to the Rust backend for game APIs, downloads, and metrics.

By default Caddy uses `deploy/Caddyfile` and publishes host port `8080`. This mode is for local testing and plain HTTP reverse proxying.

### Production HTTPS

For a server that should own `80` and `443`, use the production override:

```sh
cd deploy
cp .env.example .env
# edit .env:
# SITE_DOMAIN=your.domain.example
# ACME_EMAIL=you@example.com
# ADMIN_PASSWORD=change-me
# SECRET_KEY=change-me
# LINKPLAY_DISPLAY_HOST=your.domain.example

docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```

This uses `deploy/Caddyfile.https`. Point the domain `A`/`AAAA` record to the server and allow inbound `80/tcp`, `443/tcp`, and optionally `443/udp` for HTTP/3. Caddy stores certificates in the `caddy-data` volume, so renewals survive container recreation.

### DNS Challenge HTTPS

Use DNS challenge when port `80` cannot be exposed, or when you want wildcard certificates.

```sh
cd deploy
# edit .env:
# CADDY_TLS_MODE=dns
# CADDY_BUILD_TARGET=caddy-cloudflare
# SITE_DOMAIN=your.domain.example
# ACME_EMAIL=you@example.com
# CLOUDFLARE_API_TOKEN=...

docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```

This uses `deploy/Caddyfile.dns`, which is written for Cloudflare:

```caddyfile
tls {
	dns cloudflare {$CLOUDFLARE_API_TOKEN}
}
```

The default Caddy image does not include DNS provider plugins. `CADDY_BUILD_TARGET=caddy-cloudflare` builds the optional Caddy target with `github.com/caddy-dns/cloudflare`. For other DNS providers, change both the Dockerfile plugin and the `dns ...` line in `Caddyfile.dns`.

## Link Play

`linkplayd` runs as a separate service. UDP is published by default:

```env
LINKPLAY_DISPLAY_HOST=your.public.domain.or.ip
LINKPLAY_UDP_PUBLISHED_PORT=10900
```

The TCP control port is only published on `127.0.0.1` by default; the backend reaches it over the internal Compose network.

## Optional MinIO/S3 Mode

For S3-compatible storage with local MinIO:

```sh
cd deploy
cp .env.example .env
# edit .env:
# STORAGE_BACKEND=s3

docker compose --profile s3 up -d --build db redis minio
docker compose --profile s3 run --rm s3-sync
docker compose --profile s3 up -d app linkplayd caddy
```

The `s3-sync` command uploads `../songs` and `../bundles`, then writes `manifest.json` to the configured bucket. Run it again whenever those files change.

MinIO console is available at `http://127.0.0.1:9001/` with `MINIO_ROOT_USER` / `MINIO_ROOT_PASSWORD` from `deploy/.env`.

## Useful Commands

```sh
docker compose logs -f app
docker compose logs -f linkplayd
docker compose run --rm app init-db
docker compose --profile s3 run --rm s3-sync
docker compose down
```
