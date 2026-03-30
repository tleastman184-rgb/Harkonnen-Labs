#!/usr/bin/env sh
set -eu

BASE_DIR=${1:-"/media/earthling/Caleb's Files/harkonnen-local"}
OPENCLAW_PREFIX="$BASE_DIR/openclaw"
ANYTHINGLLM_DIR="$BASE_DIR/anythingllm"
ANYTHINGLLM_STORAGE="$ANYTHINGLLM_DIR/storage"
BIN_DIR="$BASE_DIR/bin"
COMPOSE_FILE="$ANYTHINGLLM_DIR/docker-compose.yml"
ENV_FILE="$ANYTHINGLLM_DIR/.env"
OPENCLAW_INSTALLER_URL="https://openclaw.ai/install-cli.sh"
ANYTHINGLLM_IMAGE="mintplexlabs/anythingllm:latest"
HOST_UID=$(id -u)
HOST_GID=$(id -g)

mkdir -p "$OPENCLAW_PREFIX" "$ANYTHINGLLM_STORAGE" "$BIN_DIR"

printf 'Installing OpenClaw into %s\n' "$OPENCLAW_PREFIX"
curl -fsSL "$OPENCLAW_INSTALLER_URL" | bash -s -- --prefix "$OPENCLAW_PREFIX" --no-onboard
ln -sfn "$OPENCLAW_PREFIX/bin/openclaw" "$BIN_DIR/openclaw"

printf 'Writing AnythingLLM config into %s\n' "$ANYTHINGLLM_DIR"
cat > "$ENV_FILE" <<ENV
STORAGE_LOCATION=$ANYTHINGLLM_STORAGE
CONTAINER_NAME=harkonnen-anythingllm
HOST_PORT=3001
HOST_UID=$HOST_UID
HOST_GID=$HOST_GID
ENV

cat > "$COMPOSE_FILE" <<'COMPOSE'
services:
  anythingllm:
    image: mintplexlabs/anythingllm:latest
    container_name: ${CONTAINER_NAME}
    restart: unless-stopped
    ports:
      - "${HOST_PORT}:3001"
    cap_add:
      - SYS_ADMIN
    environment:
      STORAGE_DIR: /app/server/storage
      UID: "${HOST_UID}"
      GID: "${HOST_GID}"
    volumes:
      - "${STORAGE_LOCATION}:/app/server/storage"
COMPOSE

printf 'Pulling AnythingLLM image %s\n' "$ANYTHINGLLM_IMAGE"
docker pull "$ANYTHINGLLM_IMAGE"

cat > "$BIN_DIR/anythingllm-up" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$ANYTHINGLLM_DIR"
docker compose up -d
WRAP

cat > "$BIN_DIR/anythingllm-down" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$ANYTHINGLLM_DIR"
docker compose down
WRAP

cat > "$BIN_DIR/anythingllm-logs" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$ANYTHINGLLM_DIR"
docker compose logs -f
WRAP

chmod +x "$BIN_DIR/anythingllm-up" "$BIN_DIR/anythingllm-down" "$BIN_DIR/anythingllm-logs"

printf 'Bootstrap complete.\n'
printf 'OpenClaw: %s\n' "$BIN_DIR/openclaw"
printf 'AnythingLLM up: %s\n' "$BIN_DIR/anythingllm-up"
printf 'AnythingLLM down: %s\n' "$BIN_DIR/anythingllm-down"
printf 'AnythingLLM logs: %s\n' "$BIN_DIR/anythingllm-logs"
