#!/usr/bin/env bash
# Runs the Laravel reference (the `main` branch) on http://127.0.0.1:8317
# for recording golden fixtures (scripts/laravel-reference/record.py).
#
#   scripts/laravel-reference/run.sh setup        export main, install, build assets
#   scripts/laravel-reference/run.sh start        fresh database, mock ESI, serve
#   scripts/laravel-reference/run.sh php FILE     run a PHP file in the app (stdin: data)
#   scripts/laravel-reference/run.sh sql "..."    run SQL against the reference database
#   scripts/laravel-reference/run.sh stop
#
# The checkout lives in $LARAVEL_REF_DIR (default /tmp/loganalyzer-laravel-ref),
# never inside this repository. MySQL 8 like production (case-insensitive
# collation), a mock ESI (mock_esi.py) instead of esi.evetech.net, no
# other services. Containers are named loganalyzer-laravel-*, on the
# loganalyzer-laravel network.
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
here="$repo/scripts/laravel-reference"
dir=${LARAVEL_REF_DIR:-/tmp/loganalyzer-laravel-ref}
image=loganalyzer-laravel-ref
network=loganalyzer-laravel
port=8317

run() {
  docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -e COMPOSER_HOME=/tmp/composer \
    --network "$network" -v "$dir/app:/app" "$@"
}

network_up() {
  docker network inspect "$network" >/dev/null 2>&1 || docker network create "$network" >/dev/null
}

mysql_up() {
  network_up
  if ! docker inspect loganalyzer-laravel-mysql >/dev/null 2>&1; then
    docker run -d --name loganalyzer-laravel-mysql --network "$network" \
      -e MYSQL_ROOT_PASSWORD=loganalyzer -e MYSQL_DATABASE=loganalyzer \
      mysql:8.4 --character-set-server=utf8mb4 --collation-server=utf8mb4_unicode_ci >/dev/null
  fi
  until docker exec loganalyzer-laravel-mysql mysql -uroot -ploganalyzer -e 'select 1' >/dev/null 2>&1; do sleep 1; done
}

esi_up() {
  network_up
  docker rm -f loganalyzer-laravel-esi >/dev/null 2>&1 || true
  docker run -d --name loganalyzer-laravel-esi --network "$network" \
    -v "$here/mock_esi.py:/mock_esi.py:ro" -v "$repo/tests/fixtures/esi:/esi:ro" \
    python:3-slim python3 -I /mock_esi.py /esi/names.json >/dev/null
}

case "${1:-}" in
  setup)
    docker build -t "$image" "$here"
    rm -rf "$dir" && mkdir -p "$dir/app"
    git -C "$repo" archive main | tar -x -C "$dir/app"
    cp "$here/env" "$dir/app/.env"
    # The ESI package has its base URL fixed in its own config; this copy
    # reads it from the environment, so the reference talks to the mock.
    cp "$here/esi-config.php" "$dir/app/config/esi.php"
    mysql_up
    run --name loganalyzer-laravel-setup "$image" sh -c '
      composer install --no-interaction --no-progress
      npm ci --no-audit --no-fund
      php artisan wayfinder:generate --with-form
      npx vite build'
    ;;
  start)
    mysql_up
    esi_up
    rm -f "$dir/app/storage/logs/laravel.log"
    rm -rf "$dir/app/storage/app/private/combat-logs"
    # migrate:fresh is prohibited in production (DB::prohibitDestructiveCommands),
    # so the database is recreated by hand.
    docker exec loganalyzer-laravel-mysql mysql -uroot -ploganalyzer \
      -e 'drop database if exists loganalyzer; create database loganalyzer' 2>/dev/null
    run --name loganalyzer-laravel-migrate "$image" php artisan migrate --force >/dev/null
    docker rm -f loganalyzer-laravel-ref >/dev/null 2>&1 || true
    docker run -d --name loganalyzer-laravel-ref -u "$(id -u):$(id -g)" -e HOME=/tmp \
      -e PHP_CLI_SERVER_WORKERS=4 --network "$network" \
      -v "$dir/app:/app" \
      -p "127.0.0.1:$port:$port" "$image" \
      php artisan serve --host=0.0.0.0 --port="$port" --no-reload >/dev/null
    until curl -fs "http://127.0.0.1:$port/up" >/dev/null; do sleep 0.5; done
    echo "Laravel reference on http://127.0.0.1:$port"
    ;;
  php)
    shift
    file=$1
    shift
    docker run --rm -i -u "$(id -u):$(id -g)" -e HOME=/tmp --network "$network" \
      -v "$dir/app:/app" -v "$(cd "$(dirname "$file")" && pwd)/$(basename "$file"):/script.php:ro" \
      "$image" php /script.php "$@"
    ;;
  sql)
    shift
    docker exec loganalyzer-laravel-mysql mysql -uroot -ploganalyzer --default-character-set=utf8mb4 -N -B loganalyzer -e "$*" 2>/dev/null
    ;;
  stop)
    docker rm -f loganalyzer-laravel-ref loganalyzer-laravel-mysql loganalyzer-laravel-esi >/dev/null 2>&1 || true
    docker network rm "$network" >/dev/null 2>&1 || true
    ;;
  *)
    sed -n '2,15p' "$0"
    exit 1
    ;;
esac
