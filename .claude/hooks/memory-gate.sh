#!/bin/bash
# Pre-tool: memory write audit gate (logging only — does not block).
# The sable-guard.sh script handles this via the Rust binary.
# This script exists as a standalone fallback entry point for the hook config.

BIN="${CLAUDE_PROJECT_DIR}/target/release/harkonnen-labs"

if [[ -x "$BIN" ]]; then
  # pre-write covers memory gate; re-use it
  exec "$BIN" hook pre-write
fi

# ── jq fallback ───────────────────────────────────────────────────────────────

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

if [[ "$file_path" == *"factory/memory/"* ]]; then
  log_dir="${CLAUDE_PROJECT_DIR}/factory/artifacts"
  mkdir -p "$log_dir"
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  tool=$(echo "$input" | jq -r '.tool_name // "unknown"' 2>/dev/null)
  printf '{"event":"memory_write","timestamp":"%s","tool":"%s","file":"%s"}\n' \
    "$timestamp" "$tool" "$file_path" >> "$log_dir/memory-write-audit.jsonl"
fi

exit 0
