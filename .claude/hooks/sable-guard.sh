#!/bin/bash
# Pre-tool: sable isolation guard + memory write gate.
# Delegates to the compiled Rust binary when available; falls back to jq.

BIN="${CLAUDE_PROJECT_DIR}/target/release/harkonnen-labs"

if [[ -x "$BIN" ]]; then
  exec "$BIN" hook pre-write
fi

# ── jq fallback (development / pre-build) ────────────────────────────────────

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

if [[ "$file_path" == *"factory/scenarios/hidden"* ]]; then
  echo "BLOCKED [sable-guard]: Direct writes to factory/scenarios/hidden/ are forbidden." >&2
  echo "" >&2
  echo "Hidden scenarios may only be created via the Sable scenario-generation flow." >&2
  echo "Writing directly contaminates the evaluation corpus and violates Sable's" >&2
  echo "isolation invariant (I-SB-01 / I-SB-02 in factory/agents/contracts/sable.yaml)." >&2
  exit 2
fi

if [[ "$file_path" == *"factory/memory/"* ]]; then
  log_dir="${CLAUDE_PROJECT_DIR}/factory/artifacts"
  mkdir -p "$log_dir"
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  tool=$(echo "$input" | jq -r '.tool_name // "unknown"' 2>/dev/null)
  printf '{"event":"memory_write","timestamp":"%s","tool":"%s","file":"%s"}\n' \
    "$timestamp" "$tool" "$file_path" >> "$log_dir/memory-write-audit.jsonl"
fi

exit 0
