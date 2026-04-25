#!/bin/bash
# Post-tool: append a structured audit entry for every write / bash operation.
# Delegates to the compiled Rust binary when available; falls back to jq.

BIN="${CLAUDE_PROJECT_DIR}/target/release/harkonnen-labs"

if [[ -x "$BIN" ]]; then
  exec "$BIN" hook post-audit
fi

# ── jq fallback (development / pre-build) ────────────────────────────────────

input=$(cat)
log_dir="${CLAUDE_PROJECT_DIR:-$(git -C "$(dirname "$0")" rev-parse --show-toplevel 2>/dev/null || pwd)}/factory/artifacts"
mkdir -p "$log_dir"

timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
tool=$(echo "$input" | jq -r '.tool_name // "unknown"' 2>/dev/null)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null)
command_preview=$(echo "$input" | jq -r '.tool_input.command // empty' 2>/dev/null | head -c 200)

action="${file_path:-${command_preview:-(no action captured)}}"

printf '{"timestamp":"%s","tool":"%s","action":"%s"}\n' \
  "$timestamp" "$tool" "$(echo "$action" | sed 's/"/\\"/g')" \
  >> "$log_dir/tool-audit.jsonl"

exit 0
