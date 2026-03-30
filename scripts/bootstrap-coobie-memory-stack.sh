#!/usr/bin/env sh
set -eu

BASE_DIR=${1:-"/media/earthling/Caleb's Files/harkonnen-local"}
QDRANT_DIR="$BASE_DIR/qdrant"
QDRANT_STORAGE="$QDRANT_DIR/storage"
REDIS_DIR="$BASE_DIR/redis"
REDIS_DATA="$REDIS_DIR/data"
BIN_DIR="$BASE_DIR/bin"

mkdir -p "$QDRANT_STORAGE" "$REDIS_DATA" "$BIN_DIR"

cat > "$QDRANT_DIR/docker-compose.yml" <<'COMPOSE'
services:
  qdrant:
    image: qdrant/qdrant:latest
    container_name: harkonnen-qdrant
    restart: unless-stopped
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - ./storage:/qdrant/storage
COMPOSE

cat > "$REDIS_DIR/docker-compose.yml" <<'COMPOSE'
services:
  redis:
    image: redis:7-alpine
    container_name: harkonnen-redis
    restart: unless-stopped
    ports:
      - "6379:6379"
    command: ["redis-server", "--appendonly", "yes", "--save", "60", "1"]
    volumes:
      - ./data:/data
COMPOSE

cat > "$BIN_DIR/qdrant-up" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$QDRANT_DIR"
docker compose up -d
WRAP

cat > "$BIN_DIR/qdrant-down" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$QDRANT_DIR"
docker compose down
WRAP

cat > "$BIN_DIR/qdrant-logs" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$QDRANT_DIR"
docker compose logs -f
WRAP

cat > "$BIN_DIR/redis-up" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$REDIS_DIR"
docker compose up -d
WRAP

cat > "$BIN_DIR/redis-down" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$REDIS_DIR"
docker compose down
WRAP

cat > "$BIN_DIR/redis-logs" <<WRAP
#!/usr/bin/env sh
set -eu
cd "$REDIS_DIR"
docker compose logs -f
WRAP

cat > "$BIN_DIR/coobie-memory-up" <<WRAP
#!/usr/bin/env sh
set -eu
"$BIN_DIR/qdrant-up"
"$BIN_DIR/redis-up"
WRAP

cat > "$BIN_DIR/coobie-memory-down" <<WRAP
#!/usr/bin/env sh
set -eu
"$BIN_DIR/redis-down"
"$BIN_DIR/qdrant-down"
WRAP

chmod +x   "$BIN_DIR/qdrant-up"   "$BIN_DIR/qdrant-down"   "$BIN_DIR/qdrant-logs"   "$BIN_DIR/redis-up"   "$BIN_DIR/redis-down"   "$BIN_DIR/redis-logs"   "$BIN_DIR/coobie-memory-up"   "$BIN_DIR/coobie-memory-down"

printf 'Coobie memory stack scaffolded.\n'
printf 'Qdrant compose: %s\n' "$QDRANT_DIR/docker-compose.yml"
printf 'Redis compose:  %s\n' "$REDIS_DIR/docker-compose.yml"
printf 'Launch both:    %s\n' "$BIN_DIR/coobie-memory-up"
printf 'Stop both:      %s\n' "$BIN_DIR/coobie-memory-down"
