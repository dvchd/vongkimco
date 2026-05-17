#!/bin/sh
set -e
echo "[entrypoint] Starting $(date)" >&2
mkdir -p /data /data/screenshots
chown -R app:app /data
echo "[entrypoint] Launching app as user 'app'" >&2
exec runuser -u app -- /app/vongkimco-backend "$@"
