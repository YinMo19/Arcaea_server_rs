#!/usr/bin/env sh
set -eu

command="${1:-server}"
if [ "$#" -gt 0 ]; then
    shift
fi

wait_for_tcp() {
    host="$1"
    port="$2"
    name="$3"
    retries="${4:-60}"

    i=1
    while [ "$i" -le "$retries" ]; do
        if nc -z "$host" "$port" >/dev/null 2>&1; then
            echo "$name is available at $host:$port"
            return 0
        fi
        echo "Waiting for $name at $host:$port ($i/$retries)..."
        i=$((i + 1))
        sleep 2
    done

    echo "Timed out waiting for $name at $host:$port" >&2
    return 1
}

database_host() {
    if [ -n "${DB_HOST:-}" ]; then
        printf '%s' "$DB_HOST"
        return
    fi
    printf '%s' "${DATABASE_URL:-}" | sed -E 's#^[a-zA-Z0-9+.-]+://([^@/]+@)?([^:/?]+).*#\2#'
}

database_port() {
    if [ -n "${DB_PORT:-}" ]; then
        printf '%s' "$DB_PORT"
        return
    fi
    port="$(printf '%s' "${DATABASE_URL:-}" | sed -nE 's#^[a-zA-Z0-9+.-]+://([^@/]+@)?[^:/?]+:([0-9]+).*#\2#p')"
    printf '%s' "${port:-3306}"
}

wait_for_database() {
    if [ "${WAIT_FOR_DATABASE:-true}" != "true" ]; then
        return 0
    fi
    if [ -z "${DATABASE_URL:-}" ] && [ -z "${DB_HOST:-}" ]; then
        return 0
    fi

    wait_for_tcp "$(database_host)" "$(database_port)" "database" "${WAIT_FOR_DATABASE_RETRIES:-60}"
}

wait_for_s3() {
    if [ "${WAIT_FOR_S3:-false}" != "true" ]; then
        return 0
    fi

    host="${S3_WAIT_HOST:-minio}"
    port="${S3_WAIT_PORT:-9000}"
    wait_for_tcp "$host" "$port" "s3" "${WAIT_FOR_S3_RETRIES:-60}"
}

run_db_init() {
    case "${RUN_DB_INIT:-true}" in
        true|1|yes|on)
            echo "Running init_db before server startup..."
            init_db
            ;;
    esac
}

case "$command" in
    server|arcaea-server|Arcaea_server_rs)
        wait_for_database
        run_db_init
        exec Arcaea_server_rs "$@"
        ;;
    init-db|init_db)
        wait_for_database
        exec init_db "$@"
        ;;
    linkplayd)
        exec linkplayd "$@"
        ;;
    sync-s3|sync_s3_manifest)
        wait_for_s3
        exec sync_s3_manifest "$@"
        ;;
    *)
        exec "$command" "$@"
        ;;
esac
