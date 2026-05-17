#!/bin/sh
set -e
echo "[entrypoint] Starting $(date)" >&2
echo "[entrypoint] Shared lib check:" >&2
ldd /app/vongkimco-backend >&2 || echo "[entrypoint] ldd failed!" >&2
mkdir -p /data /data/screenshots
chown -R app:app /data
echo "[entrypoint] Launching app as user 'app'" >&2
exec runuser -u app -- /app/vongkimco-backend "$@"
