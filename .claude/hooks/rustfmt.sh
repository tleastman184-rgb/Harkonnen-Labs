#!/bin/bash
# Post-tool: run rustfmt on .rs files after any edit.
# Delegates to the compiled Rust binary when available; falls back to direct rustfmt call.

BIN="${CLAUDE_PROJECT_DIR}/target/release/harkonnen-labs"

if [[ -x "$BIN" ]]; then
  exec "$BIN" hook post-format
fi

# ── direct fallback (development / pre-build) ─────────────────────────────────

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

if [[ "$file_path" == *.rs ]] && command -v rustfmt &>/dev/null; then
  rustfmt --edition 2021 "$file_path" 2>/dev/null
fi

exit 0
