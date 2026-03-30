#!/usr/bin/env sh
set -eu

BASE_DIR=${HARKONNEN_LOCAL_DIR:-"/media/earthling/Caleb's Files/harkonnen-local"}
exec "$BASE_DIR/bin/openclaw" "$@"
