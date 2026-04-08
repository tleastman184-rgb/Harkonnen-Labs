#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
RUNNER_PATH="$REPO_ROOT/scripts/run-tau2-harkonnen.sh"
NAME="tau2-bench"
COMMAND_VAR="TAU2_BENCH_COMMAND"
ROOT_VAR="TAU2_BENCH_ROOT"
COMMAND="${TAU2_BENCH_COMMAND:-}"
ROOT="${TAU2_BENCH_ROOT:-}"
AUTOSTART="${TAU2_BENCH_AUTOSTART_HARKONNEN:-0}"
PORT="${TAU2_BENCH_API_PORT:-3000}"
BASE_URL="${TAU2_BENCH_HARKONNEN_BASE_URL:-http://127.0.0.1:${PORT}}"
HEALTH_PATH="${TAU2_BENCH_HEALTH_PATH:-/api/setup/check}"
WAIT_SECS="${TAU2_BENCH_WAIT_SECS:-90}"
SERVER_LOG="${TAU2_BENCH_SERVER_LOG:-/tmp/harkonnen-tau2-server.log}"
SERVER_PID=""

if [ -z "$COMMAND" ]; then
  printf '%s adapter not configured. Set %s to the command that runs Harkonnen on this benchmark. Optionally set %s to the cloned benchmark repo root.\n' "$NAME" "$COMMAND_VAR" "$ROOT_VAR" >&2
  exit 10
fi

cleanup() {
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

if [ -n "$ROOT" ]; then
  cd "$ROOT"
fi

if [ "$AUTOSTART" = "1" ]; then
  cargo run -- serve --port "$PORT" >"$SERVER_LOG" 2>&1 &
  SERVER_PID=$!

  i=0
  while [ "$i" -lt "$WAIT_SECS" ]; do
    if curl -fsS "$BASE_URL$HEALTH_PATH" >/dev/null 2>&1; then
      break
    fi
    i=$((i + 1))
    sleep 1
  done

  if ! curl -fsS "$BASE_URL$HEALTH_PATH" >/dev/null 2>&1; then
    printf '%s failed to start Harkonnen API at %s%s within %ss. See %s\n' "$NAME" "$BASE_URL" "$HEALTH_PATH" "$WAIT_SECS" "$SERVER_LOG" >&2
    exit 1
  fi
fi

export TAU2_BENCH_HARKONNEN_BASE_URL="$BASE_URL"
export HARKONNEN_BENCH_BASE_URL="$BASE_URL"
export TAU2_BENCH_HARKONNEN_REPO_ROOT="$REPO_ROOT"
export TAU2_BENCH_HARKONNEN_RUNNER="$RUNNER_PATH"

exec /bin/sh -lc "$COMMAND"
