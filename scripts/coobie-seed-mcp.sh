#!/usr/bin/env sh
# Coobie — MCP memory backend seed helper (all setups)
#
# The @modelcontextprotocol/server-memory server holds entities in process memory.
# This script reads factory/memory/index.json (built by `harkonnen memory init`)
# and prints the Claude Code MCP config block plus entity payloads that can be
# pushed to the server on first run.
#
# Usage:
#   cargo run -- memory init         # build index.json first
#   ./scripts/coobie-seed-mcp.sh

set -eu

INDEX="./factory/memory/index.json"
MEMORY_DIR="./factory/memory"

if [ ! -f "$INDEX" ]; then
    echo "ERROR: $INDEX not found."
    echo "  Run first: cargo run -- memory init"
    exit 1
fi

ENTRY_COUNT=$(grep -c '"id"' "$INDEX" 2>/dev/null || echo "0")

# ── 1. Claude Code MCP config block ──────────────────────────────────────────

cat <<'CONFIG'
── Claude Code MCP integration ─────────────────────────────────────────────────

Add the following to .claude/settings.local.json (or ~/.claude/settings.json)
so Claude Code can call the memory server as Coobie:

{
  "mcpServers": {
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"],
      "env": {}
    },
    "filesystem": {
      "command": "npx",
      "args": [
        "-y", "@modelcontextprotocol/server-filesystem",
        "./products",
        "./factory/workspaces",
        "./factory/artifacts",
        "./factory/memory"
      ]
    }
  }
}

CONFIG

# ── 2. Report what would be seeded ───────────────────────────────────────────

printf '── Memory index (%s entries) ───────────────────────────────────────────────\n\n' "$ENTRY_COUNT"

# Print id + summary for each entry using basic field extraction
awk '
/"id":/ { id = $0; gsub(/.*"id": "|",.*/, "", id) }
/"summary":/ {
    summary = $0
    gsub(/.*"summary": "|",.*/, "", summary)
    printf "  %-30s %s\n", id, summary
}
' "$INDEX"

echo ""

# ── 3. Persistence note ───────────────────────────────────────────────────────

cat <<'NOTE'
── Persistence ─────────────────────────────────────────────────────────────────

server-memory stores entities in process RAM — they reset on server restart.
For cross-session persistence across restarts, set in your .env or shell:

  MEMORY_FILE_PATH=./factory/memory/store.json

Then the server reads/writes that file on startup/shutdown.
The file-backed index at factory/memory/index.json is always authoritative;
run `harkonnen memory index` after adding new .md files.

NOTE

# ── 4. On-demand seeding ─────────────────────────────────────────────────────

cat <<'SEED'
── On-demand seeding ────────────────────────────────────────────────────────────

After the MCP memory server is running (via Claude Code or npx directly),
tell Coobie to load its context by sending:

  "Coobie, load your memory from factory/memory/index.json"

Coobie will use the mcp:memory tool to create entities for each indexed entry.
You can also ask Coobie to retrieve context directly from the md files via
mcp:filesystem, which does not require the memory server to be pre-seeded.

SEED
