#!/usr/bin/env sh
# Harkonnen Labs — Linux Factory Bring-Up
# Single script to go from fresh clone to a running factory spine on home-linux.
#
# Run from the repo root:
#   ./scripts/factory-up-linux.sh
#
# Idempotent — safe to re-run. Existing .env and seed docs are not overwritten.
#
# What this script does:
#   1.  Check prerequisites (node, npm, cargo, docker)
#   2.  Resolve API keys (.env)
#   3.  Set HARKONNEN_SETUP=home-linux for this session
#   4.  Install MCP server npm packages
#   5.  Build the harkonnen binary
#   6.  Run: harkonnen memory init
#   7.  Run: harkonnen setup check
#   8.  Write .claude/settings.local.json MCP block
#   9.  Start AnythingLLM (if Docker is available)
#   10. Seed AnythingLLM (if running)
#   11. Print status summary

set -eu
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# ── Helpers ───────────────────────────────────────────────────────────────────

step() { printf '\n  >> %s\n' "$1"; }
ok()   { printf '     [ok] %s\n' "$1"; }
warn() { printf '     [!!] %s\n' "$1"; }
info() { printf '          %s\n' "$1"; }
fail() { printf '     [XX] %s\n\n' "$1"; exit 1; }

require() {
    cmd="$1"; hint="$2"
    if ! command -v "$cmd" > /dev/null 2>&1; then
        fail "Missing: $cmd  →  $hint"
    fi
    ok "$cmd $($cmd --version 2>&1 | head -1)"
}

# ── Banner ────────────────────────────────────────────────────────────────────

printf '\n'
printf '  ╔══════════════════════════════════════════════╗\n'
printf '  ║   Harkonnen Labs — Linux Factory Bring-Up    ║\n'
printf '  ╚══════════════════════════════════════════════╝\n'
printf '  Setup: home-linux (Claude + Gemini + Codex)\n'
printf '  Root:  %s\n' "$REPO_ROOT"

# ── 1. Prerequisites ──────────────────────────────────────────────────────────

step "Checking prerequisites"
require "node"  "https://nodejs.org"
require "npm"   "bundled with Node.js"
require "cargo" "https://rustup.rs"

DOCKER_OK=false
if command -v docker > /dev/null 2>&1; then
    ok "docker $(docker --version 2>&1 | head -1)"
    DOCKER_OK=true
else
    warn "docker not found — AnythingLLM step will be skipped"
fi

# ── 2. Load .env ──────────────────────────────────────────────────────────────

step "Loading .env"

ENV_FILE="$REPO_ROOT/.env"
if [ ! -f "$ENV_FILE" ]; then
    if [ -f "$REPO_ROOT/.env.example" ]; then
        cp "$REPO_ROOT/.env.example" "$ENV_FILE"
        ok "Created .env from .env.example — edit it to set API keys"
    else
        printf 'HARKONNEN_SETUP=home-linux\n' > "$ENV_FILE"
        ok "Created minimal .env"
    fi
else
    ok ".env exists"
fi

# Source .env (skip comment lines and lines without =)
while IFS= read -r line; do
    case "$line" in
        '#'*|'') continue ;;
        *=*)
            key="${line%%=*}"
            val="${line#*=}"
            # Only set if not already in environment
            eval "[ -z \"\${${key}+x}\" ] && export ${key}=\"${val}\" || true"
            ;;
    esac
done < "$ENV_FILE"

# Ensure HARKONNEN_SETUP is set in the file
if ! grep -q "^HARKONNEN_SETUP=" "$ENV_FILE" 2>/dev/null; then
    printf '\nHARKONNEN_SETUP=home-linux\n' >> "$ENV_FILE"
    info "Added HARKONNEN_SETUP=home-linux to .env"
fi

# ── 3. Session environment ────────────────────────────────────────────────────

step "Setting session environment"
export HARKONNEN_SETUP=home-linux
ok "HARKONNEN_SETUP=home-linux"

# Warn on missing API keys (don't fail — setup check will report them)
for var in ANTHROPIC_API_KEY GEMINI_API_KEY OPENAI_API_KEY; do
    eval val="\${${var}:-}"
    if [ -z "$val" ]; then
        warn "$var is not set (edit .env or export it before running)"
    else
        ok "$var is set"
    fi
done

# ── 4. Install MCP packages ───────────────────────────────────────────────────

step "Installing MCP server packages"

for pkg in \
    "@modelcontextprotocol/server-filesystem" \
    "@modelcontextprotocol/server-memory" \
    "@modelcontextprotocol/server-sqlite" \
    "@modelcontextprotocol/server-github" \
    "@modelcontextprotocol/server-brave-search"
do
    if npm list -g --depth=0 2>/dev/null | grep -q "$pkg"; then
        ok "$pkg (already installed)"
    else
        info "Installing $pkg ..."
        npm install --global "$pkg" --silent
        ok "$pkg"
    fi
done

# ── 5. Build harkonnen binary ─────────────────────────────────────────────────

step "Building harkonnen binary"

RELEASE_FLAG=""
if [ "${1:-}" = "--release" ]; then
    RELEASE_FLAG="--release"
    info "cargo build --release"
else
    info "cargo build  (pass --release for an optimized build)"
fi

cargo build $RELEASE_FLAG 2>&1

if [ -n "$RELEASE_FLAG" ]; then
    BIN="$REPO_ROOT/target/release/harkonnen"
else
    BIN="$REPO_ROOT/target/debug/harkonnen"
fi

[ -f "$BIN" ] || fail "Build failed — binary not found at $BIN"
ok "Binary: $BIN"

# ── 6. Memory init (Coobie) ───────────────────────────────────────────────────

step "Initializing Coobie's memory  (harkonnen memory init)"
"$BIN" memory init

# ── 7. Setup check ────────────────────────────────────────────────────────────

step "Verifying setup  (harkonnen setup check)"
printf '\n'
"$BIN" setup check
printf '\n'

# ── 8. Claude Code MCP config ────────────────────────────────────────────────

step "Writing .claude/settings.local.json (MCP server config)"

CLAUDE_DIR="$REPO_ROOT/.claude"
CLAUDE_SETTINGS="$CLAUDE_DIR/settings.local.json"
mkdir -p "$CLAUDE_DIR"

MCP_JSON='{
  "mcpServers": {
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"],
      "env": { "MEMORY_FILE_PATH": "./factory/memory/store.json" }
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
    },
    "sqlite": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-sqlite", "./factory/state.db"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_PERSONAL_ACCESS_TOKEN": "'"${GITHUB_TOKEN:-}"'" }
    },
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "env": { "BRAVE_API_KEY": "'"${BRAVE_API_KEY:-}"'" }
    }
  }
}'

if [ -f "$CLAUDE_SETTINGS" ]; then
    # Settings exist — use Python or node to merge if available, else warn
    if command -v python3 > /dev/null 2>&1; then
        python3 - "$CLAUDE_SETTINGS" "$MCP_JSON" <<'PY'
import sys, json
path = sys.argv[1]
new_mcp = json.loads(sys.argv[2]).get("mcpServers", {})
with open(path) as f:
    existing = json.load(f)
existing.setdefault("mcpServers", {}).update(new_mcp)
with open(path, "w") as f:
    json.dump(existing, f, indent=2)
PY
        ok "Merged MCP servers into existing $CLAUDE_SETTINGS"
    else
        warn "$CLAUDE_SETTINGS already exists — MCP block NOT merged (no python3 to merge safely)"
        info "Manually add the mcpServers block from .claude/mcp-block.json"
        printf '%s\n' "$MCP_JSON" > "$CLAUDE_DIR/mcp-block.json"
    fi
else
    printf '%s\n' "$MCP_JSON" > "$CLAUDE_SETTINGS"
    ok "Created $CLAUDE_SETTINGS"
fi

# Also write the permissions block if the file is new or missing permissions
if ! grep -q '"allow"' "$CLAUDE_SETTINGS" 2>/dev/null; then
    python3 - "$CLAUDE_SETTINGS" "$REPO_ROOT" <<'PY' 2>/dev/null || true
import sys, json
path, root = sys.argv[1], sys.argv[2]
with open(path) as f:
    cfg = json.load(f)
cfg.setdefault("permissions", {}).setdefault("allow", [f"Read({root}/**)"])
with open(path, "w") as f:
    json.dump(cfg, f, indent=2)
PY
fi

# ── 9. AnythingLLM ───────────────────────────────────────────────────────────

step "AnythingLLM (home-linux, Docker-based RAG)"

if [ "$DOCKER_OK" = "true" ]; then
    ANYTHINGLLM_DIR="${ANYTHINGLLM_DIR:-$HOME/harkonnen-local/anythingllm}"
    UP_CMD="$HOME/harkonnen-local/bin/anythingllm-up"

    if [ -f "$UP_CMD" ]; then
        info "Starting AnythingLLM ..."
        "$UP_CMD" && ok "AnythingLLM started on http://localhost:3001" || warn "AnythingLLM start failed (check Docker)"

        if [ -n "${ANYTHINGLLM_API_KEY:-}" ]; then
            info "Seeding Coobie documents into AnythingLLM ..."
            ./scripts/coobie-seed-anythingllm.sh && ok "AnythingLLM seeded" || warn "Seeding failed (check ANYTHINGLLM_API_KEY)"
        else
            warn "ANYTHINGLLM_API_KEY not set — skipping document upload"
            info "After setting the key, run: ./scripts/coobie-seed-anythingllm.sh"
        fi
    else
        warn "AnythingLLM not bootstrapped yet"
        info "Run first: ./scripts/bootstrap-local-stack.sh"
    fi
else
    info "Skipped (Docker not available)"
fi

# ── 10. Summary ───────────────────────────────────────────────────────────────

printf '\n'
printf '  ════════════════════════════════════════════════\n'
printf '  Factory spine is up.\n'
printf '  ════════════════════════════════════════════════\n'
printf '\n'

printf '  Working now:\n'
for item in \
    "harkonnen spec validate <file>          — Scout: parse specs" \
    "harkonnen run start <spec> --product X  — create run + workspace" \
    "harkonnen run status/report <id>        — inspect runs" \
    "harkonnen artifact package <id>         — package outputs" \
    "harkonnen memory init / index           — Coobie: seed + rebuild index" \
    "harkonnen setup check                   — verify providers + MCP" \
    "MCP: memory, filesystem, sqlite, github, brave-search"
do
    printf '    [+] %s\n' "$item"
done

printf '\n  Not yet wired (next build layer):\n'
for item in \
    "Real LLM calls per agent   — profiles exist, execution adapters pending" \
    "Hidden scenarios (Sable)   — isolation layer planned" \
    "Digital twins (Ash)        — provisioning planned" \
    "Pack Board web UI          — axum server stubbed, not yet built"
do
    printf '    [ ] %s\n' "$item"
done

printf '\n  Next steps:\n'
printf '    1. Restart Claude Code — it will pick up the new MCP servers\n'
printf '    2. Tell Coobie: "load your memory from factory/memory/index.json"\n'
printf '    3. harkonnen spec validate factory/specs/examples/sample_feature.yaml\n'
printf '    4. harkonnen run start factory/specs/examples/sample_feature.yaml --product sample-app\n'
printf '\n  Binary:  %s\n' "$BIN"
printf '  Setup:   %s\n' "$HARKONNEN_SETUP"
printf '  Memory:  %s/factory/memory/\n\n' "$REPO_ROOT"
