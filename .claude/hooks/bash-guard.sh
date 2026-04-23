#!/bin/bash
# Pre-tool: destructive bash command guard.
# Delegates to the compiled Rust binary when available; falls back to jq.

BIN="${CLAUDE_PROJECT_DIR}/target/release/harkonnen-labs"

if [[ -x "$BIN" ]]; then
  exec "$BIN" hook pre-bash
fi

# ── jq fallback (development / pre-build) ────────────────────────────────────

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)

[[ -z "$command" ]] && exit 0

declare -a BLOCKED=(
  "rm -rf" "rm -fr"
  "git push --force" "git push -f"
  "git reset --hard"
  "git clean -f"
  "DROP TABLE" "DROP DATABASE" "DROP SCHEMA"
  "> factory/state.db"
)

for pattern in "${BLOCKED[@]}"; do
  if echo "$command" | grep -qF "$pattern"; then
    echo "BLOCKED [bash-guard]: Destructive command pattern detected." >&2
    echo "  Pattern: '$pattern'" >&2
    echo "  Command: ${command:0:200}" >&2
    echo "" >&2
    echo "Confirm explicitly with the operator before re-attempting." >&2
    exit 2
  fi
done

exit 0
