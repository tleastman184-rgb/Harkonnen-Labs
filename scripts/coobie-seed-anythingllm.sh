#!/usr/bin/env sh
# Coobie — AnythingLLM backend seed script (home-linux only)
#
# Creates a 'harkonnen-factory' workspace in AnythingLLM and uploads
# memory markdown plus imported assets recursively from factory/memory/.
#
# Prerequisites:
#   - AnythingLLM running (./bin/anythingllm-up)
#   - ANYTHINGLLM_API_KEY set (Admin > API Keys in the AnythingLLM UI)
#
# Usage:
#   export ANYTHINGLLM_API_KEY=<your-key>
#   ./scripts/coobie-seed-anythingllm.sh
#
# Optional overrides:
#   ANYTHINGLLM_BASE_URL (default: http://localhost:3001/api)
#   MEMORY_DIR           (default: ./factory/memory)

set -eu

BASE_URL="${ANYTHINGLLM_BASE_URL:-http://localhost:3001/api}"
MEMORY_DIR="${MEMORY_DIR:-./factory/memory}"
WORKSPACE_NAME="harkonnen-factory"
UPLOADS_FILE=$(mktemp)
trap 'rm -f "$UPLOADS_FILE"' EXIT INT TERM

if [ -z "${ANYTHINGLLM_API_KEY:-}" ]; then
    echo "ERROR: ANYTHINGLLM_API_KEY is not set."
    echo "  Open AnythingLLM > Admin > API Keys and create one."
    exit 1
fi

mime_for_path() {
    case "${1##*.}" in
        md) echo "text/markdown" ;;
        txt) echo "text/plain" ;;
        csv) echo "text/csv" ;;
        pdf) echo "application/pdf" ;;
        png) echo "image/png" ;;
        jpg|jpeg) echo "image/jpeg" ;;
        gif) echo "image/gif" ;;
        webp) echo "image/webp" ;;
        svg) echo "image/svg+xml" ;;
        doc) echo "application/msword" ;;
        docx) echo "application/vnd.openxmlformats-officedocument.wordprocessingml.document" ;;
        ppt) echo "application/vnd.ms-powerpoint" ;;
        pptx) echo "application/vnd.openxmlformats-officedocument.presentationml.presentation" ;;
        xls) echo "application/vnd.ms-excel" ;;
        xlsx) echo "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" ;;
        *) echo "application/octet-stream" ;;
    esac
}

AUTH_HEADER="Authorization: Bearer $ANYTHINGLLM_API_KEY"

printf 'Creating workspace: %s\n' "$WORKSPACE_NAME"

WORKSPACE_RESPONSE=$(curl -sf -X POST \
    "$BASE_URL/v1/workspace/new" \
    -H "$AUTH_HEADER" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"$WORKSPACE_NAME\"}" 2>&1) || true

if echo "$WORKSPACE_RESPONSE" | grep -q '"slug"'; then
    SLUG=$(echo "$WORKSPACE_RESPONSE" | grep -o '"slug":"[^"]*"' | head -1 | sed 's/"slug":"//;s/"//')
    printf 'Workspace ready: %s (slug: %s)\n\n' "$WORKSPACE_NAME" "$SLUG"
else
    SLUG=$(curl -sf \
        "$BASE_URL/v1/workspaces" \
        -H "$AUTH_HEADER" | grep -o '"slug":"[^"]*"' | head -1 | sed 's/"slug":"//;s/"//')
    if [ -z "$SLUG" ]; then
        echo "ERROR: Could not create or find workspace."
        echo "Response: $WORKSPACE_RESPONSE"
        exit 1
    fi
    printf 'Using existing workspace slug: %s\n\n' "$SLUG"
fi

printf 'Uploading memory files from %s\n' "$MEMORY_DIR"

find "$MEMORY_DIR" -type f | sort | while IFS= read -r filepath; do
    case "$filepath" in
        */index.json|*/store.json) continue ;;
    esac

    filename=$(basename "$filepath")
    mime=$(mime_for_path "$filepath")

    printf '  uploading %s ... ' "$filename"
    UPLOAD_RESPONSE=$(curl -sf -X POST \
        "$BASE_URL/v1/document/upload" \
        -H "$AUTH_HEADER" \
        -F "file=@$filepath;type=$mime" 2>&1) || true

    if echo "$UPLOAD_RESPONSE" | grep -q '"location"'; then
        LOCATION=$(echo "$UPLOAD_RESPONSE" | grep -o '"location":"[^"]*"' | head -1 | sed 's/"location":"//;s/"//')
        printf 'ok (%s)\n' "$LOCATION"
        printf '%s\n' "$LOCATION" >> "$UPLOADS_FILE"
    else
        printf 'WARN (already exists or upload failed)\n'
    fi
done

if [ -s "$UPLOADS_FILE" ]; then
    printf '\nEmbedding documents into workspace %s ...\n' "$SLUG"
    ADDS=$(sed 's/.*/"&"/' "$UPLOADS_FILE" | paste -sd ',' -)

    curl -sf -X POST \
        "$BASE_URL/v1/workspace/$SLUG/update-embeddings" \
        -H "$AUTH_HEADER" \
        -H "Content-Type: application/json" \
        -d "{\"adds\": [$ADDS], \"deletes\": []}" > /dev/null

    printf 'Embedding complete.\n'
else
    printf '\nNo new documents to embed.\n'
fi

printf '\nAnythingLLM backend ready.\n'
printf 'Workspace: %s  (slug: %s)\n' "$WORKSPACE_NAME" "$SLUG"
printf 'Coobie can now retrieve context via AnythingLLM plus the local file-backed index.\n'
printf '\nNext: set ANYTHINGLLM_WORKSPACE_SLUG=%s in .env\n' "$SLUG"
