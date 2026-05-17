#!/bin/sh
echo "[entrypoint] Starting $(date)" >&2
echo "[entrypoint] Binary perms:" >&2
ls -la /app/vongkimco-backend >&2
echo "[entrypoint] /data perms:" >&2
ls -la /data 2>&1 >&2 || echo "[entrypoint] /data missing" >&2
mkdir -p /data /data/screenshots
chown -R app:app /data
echo "[entrypoint] Launching app as user 'app'" >&2
runuser -u app -- /app/vongkimco-backend "$@"
EXIT_CODE=$?
echo "[entrypoint] App exited with code: $EXIT_CODE" >&2
exit $EXIT_CODE
