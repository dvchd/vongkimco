#!/bin/sh
set -e
mkdir -p /data /data/screenshots
chown -R app:app /data
exec runuser -u app -- /app/vongkimco-backend "$@"
