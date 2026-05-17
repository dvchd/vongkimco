#!/bin/sh
set -e
mkdir -p /data /data/screenshots
chown -R app:app /data
exec su-exec app /app/vongkimco-backend "$@"
