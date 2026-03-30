#!/usr/bin/env sh
set -eu

BASE_DIR=${HARKONNEN_LOCAL_DIR:-"/media/earthling/Caleb's Files/harkonnen-local"}
ANYTHINGLLM_DIR="$BASE_DIR/anythingllm"
QDRANT_DIR="$BASE_DIR/qdrant"
REDIS_DIR="$BASE_DIR/redis"
OPENCLAW_BIN="$BASE_DIR/bin/openclaw"

printf 'OpenClaw binary: '
if [ -x "$OPENCLAW_BIN" ]; then
  printf '%s\n' "$OPENCLAW_BIN"
  "$OPENCLAW_BIN" --version || true
else
  printf 'missing\n'
fi

printf 'AnythingLLM container status:\n'
if [ -d "$ANYTHINGLLM_DIR" ]; then
  (cd "$ANYTHINGLLM_DIR" && docker compose ps) || true
else
  printf 'AnythingLLM directory missing\n'
fi


printf 'Qdrant container status:\n'
if [ -d "$QDRANT_DIR" ]; then
  (cd "$QDRANT_DIR" && docker compose ps) || true
else
  printf 'Qdrant directory missing\n'
fi

printf 'Redis container status:\n'
if [ -d "$REDIS_DIR" ]; then
  (cd "$REDIS_DIR" && docker compose ps) || true
else
  printf 'Redis directory missing\n'
fi
