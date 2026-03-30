# Harkonnen Labs — Agent Context

This file is the universal context document for all AI agents (Claude, Gemini, Codex)
working in this repository. Read it before touching any code or spec.

For structured machine-readable data, see `factory/context/`.
For memory retrieval, see `factory/memory/` and ask Coobie.

---

## What This System Is

A local-first, spec-driven AI software factory. The human defines intent in YAML specs.
A pack of nine specialist agents performs the implementation inside a constrained,
observable factory. Correctness is judged by behavioral outcomes, not code review.

**The factory is not a chatbot. It is a machine room.**

---

## Codebase Map

```
src/                    Rust CLI (cargo run -- <command>)
  main.rs               Entry point and command dispatch
  cli.rs                All subcommands and handlers
  config.rs             Path discovery + SetupConfig loading
  setup.rs              SetupConfig structs, provider resolution, routing
  orchestrator.rs       AppContext, run lifecycle
  memory.rs             File-backed memory store, init, reindex, retrieve
  spec.rs               YAML spec loader
  models.rs             Shared data types (Spec, RunRecord, IntentPackage)
  policy.rs             Path boundary enforcement
  workspace.rs          Per-run workspace creation
  db.rs                 SQLite init
  reporting.rs          Run report generation

factory/
  agents/profiles/      Nine agent YAML profiles (one per agent)
  agents/personality/   labrador.md — shared personality for all agents
  memory/               Coobie's memory store (md files + index.json)
  mcp/                  MCP server documentation YAMLs
  context/              Machine-parseable YAML context for agent consumption
  specs/                Factory input specs (YAML)
  scenarios/            Hidden behavioral scenarios (Sable + Keeper only)
  workspaces/           Per-run isolated workspaces
  artifacts/            Packaged run outputs
  logs/                 Run logs
  state.db              SQLite run metadata

setups/                 Named environment TOML files
harkonnen.toml          Active/default setup config
.env.example            Environment variable template
```

---

## CLI Commands

```sh
cargo run -- spec validate <file>              # Scout: parse and validate a spec
cargo run -- run start <spec> --product <name> # Start a factory run
cargo run -- run status <run-id>               # Check run status
cargo run -- run report <run-id>               # Print run report
cargo run -- artifact package <run-id>         # Package artifacts for a run
cargo run -- memory init                       # Seed Coobie's memory, print backend setup
cargo run -- memory index                      # Rebuild index.json from md files
cargo run -- setup check                       # Verify active setup (providers + MCP)
```

---

## Agent Roster

Nine specialist agents, each with a bounded role, permitted tools, and a provider assignment.
Agent profiles live in `factory/agents/profiles/<name>.yaml`.

| Agent   | Role                | Profile Provider | Key Responsibility                         |
|---------|---------------------|------------------|--------------------------------------------|
| Scout   | Spec retriever      | claude (pinned)  | Parse specs, flag ambiguity, produce intent package |
| Mason   | Build retriever     | default          | Generate and modify code, multi-file changes |
| Piper   | Tool retriever      | default          | Run build tools, fetch docs, execute helpers |
| Bramble | Test retriever      | default          | Generate tests, run lint/build/visible tests |
| Sable   | Scenario retriever  | claude (pinned)  | Execute hidden scenarios, produce eval reports |
| Ash     | Twin retriever      | default          | Provision digital twins, mock dependencies |
| Flint   | Artifact retriever  | default          | Collect outputs, package artifact bundles |
| Coobie  | Memory retriever    | default          | Retrieve prior specs/patterns, store knowledge |
| Keeper  | Boundary retriever  | claude (pinned)  | Enforce policy, prevent unsafe actions, protect secrets |

**Pinned to Claude**: Scout, Sable, Keeper — these are trust-critical roles.
**Routable**: Mason, Piper, Bramble, Ash, Flint, Coobie — provider set per setup.

### Key Invariants

- Mason **cannot** access `scenario_store` — prevents test gaming
- Sable **cannot** write implementation code
- Only Keeper has `policy_engine` access
- All agents share the labrador personality: loyal, honest, persistent, never bluffs

---

## Setup System

The active setup is read from (in order):
1. `HARKONNEN_SETUP=work-windows` → `setups/work-windows.toml`
2. `HARKONNEN_SETUP=./path/to/file.toml` → that file directly
3. `harkonnen.toml` (repo root)
4. Built-in default (Claude only)

### Provider Routing

Setup files control which AI model each agent uses:

```toml
[providers]
default = "gemini"           # agents with provider: default use this

[routing.agents]
coobie = "claude"            # Coobie always uses Claude on this machine
mason  = "codex"             # Mason uses Codex for code generation
# all others inherit providers.default = gemini
```

Agent profiles declare their preferred provider (`claude`, `default`, etc.).
Setup `[routing.agents]` overrides that declaration for a specific machine.
This means **agent profiles stay stable; routing is per-environment**.

### Provider Fields

```toml
[providers.claude]
type         = "anthropic"
model        = "claude-sonnet-4-6"
api_key_env  = "ANTHROPIC_API_KEY"
enabled      = true
usage_rights = "standard"    # standard | high | targeted
surface      = "claude-code" # which surface/tool runs this provider
```

### Setup Variants

| Setup            | Providers           | Default | Docker | AnythingLLM |
|------------------|---------------------|---------|--------|-------------|
| home-linux       | Claude+Gemini+Codex | gemini  | Yes    | Yes         |
| work-windows     | Claude only         | claude  | No     | No          |
| ci               | Claude Haiku only   | claude  | No     | No          |

---

## MCP Integration

MCP servers back the abstract tool names in agent profiles. Configured per setup in
`[[mcp.servers]]` blocks. The `tool_aliases` field connects abstract names to real servers.

### Active Servers (home-linux)

| Server       | Package                               | Aliases                              |
|--------------|---------------------------------------|--------------------------------------|
| filesystem   | @modelcontextprotocol/server-filesystem | filesystem_read, workspace_write, artifact_writer |
| memory       | @modelcontextprotocol/server-memory   | memory_store, metadata_query         |
| sqlite       | @modelcontextprotocol/server-sqlite   | db_read, metadata_query              |
| github       | @modelcontextprotocol/server-github   | fetch_docs, github_read              |
| brave-search | @modelcontextprotocol/server-brave-search | fetch_docs, web_search           |

### Adding a New MCP Server

1. Add `[[mcp.servers]]` to the active setup TOML
2. Create `factory/mcp/<name>.yaml` (documentation + alias list)
3. Reference the tool alias in the relevant agent profile's `allowed_tools`
4. Run `cargo run -- setup check` to verify the command is on PATH

### Claude Code MCP Config

To wire MCP servers into Claude Code directly, add to `.claude/settings.local.json`:

```json
{
  "mcpServers": {
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"]
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem",
               "./products", "./factory/workspaces", "./factory/artifacts", "./factory/memory"]
    }
  }
}
```

---

## Coobie — Memory Agent

Coobie manages the factory's accumulated knowledge. Two backends, one source of truth.

### Source of Truth: `factory/memory/`

All memory lives as `*.md` files with YAML frontmatter:
```markdown
---
tags: [spec, auth, jwt]
summary: JWT auth pattern used in sample-app
---
Content here...
```

`harkonnen memory index` scans these and builds `factory/memory/index.json`.

### Backend 1: File Index (all setups)
Keyword search over `index.json`. Built into the Rust layer. Always available.

### Backend 2: MCP Memory Server (all setups)
`@modelcontextprotocol/server-memory` — fast key-value entity store.
Coobie pushes key facts here for rapid retrieval during runs.
Set `MEMORY_FILE_PATH=./factory/memory/store.json` for persistence across restarts.

### Backend 3: AnythingLLM (home-linux only)
Docker-based RAG over the `factory/memory/` directory.
Provides semantic search on large document collections.
Seed with: `./scripts/coobie-seed-anythingllm.sh`

### Initializing Coobie

```sh
cargo run -- memory init          # write seed docs + build index
./scripts/coobie-seed-mcp.sh      # print MCP config + seeding instructions
./scripts/coobie-seed-anythingllm.sh  # home-linux only: upload to AnythingLLM
```

### Adding Memory

To store a new fact in Coobie's memory, either:
- Write a `.md` file to `factory/memory/` and run `harkonnen memory index`
- Ask Coobie directly during a run: "Coobie, store this pattern for future runs"

---

## Spec Format

All factory runs start with a YAML spec in `factory/specs/`. Required fields:

```yaml
id: snake_case_identifier
title: Human-readable name
purpose: One-sentence intent
scope: [list of in-scope items]
constraints: [list of things that must not happen]
inputs: [what the factory receives]
outputs: [what the factory produces]
acceptance_criteria: [visible pass/fail conditions]
forbidden_behaviors: [must-never-occur items]
rollback_requirements: [what survives a run failure]
dependencies: [external packages or services]
performance_expectations: [timing or throughput targets]
security_expectations: [auth, secrets, isolation]
```

---

## Development Conventions

- **Read first**: understand existing code before suggesting changes
- **Specs before code**: if there is no spec, write one first
- **Boundary discipline**: never let factory code reach into `factory/scenarios/` (Sable only)
- **Memory discipline**: after any run that produces a reusable pattern, store it via Coobie
- **Setup portability**: never hardcode paths or provider names in Rust source — use `SetupConfig`
- **MCP first**: prefer registering a new capability as an MCP server over adding Rust code

---

## What Is and Is Not Implemented

### Implemented
- Spec loading and validation (Scout layer)
- Run creation, status, and persistence in SQLite
- Per-run workspace isolation
- Artifact packaging
- File-backed memory store with keyword retrieval (Coobie)
- `setup check` command with provider and MCP server verification
- Agent profile loading and provider routing display
- Bootstrap scripts for home-linux and work-windows

### Planned (next build layer)
- Agent execution adapters (real LLM calls per agent, per provider)
- Phase state machine in the orchestrator
- Hidden scenario isolation and execution (Sable)
- Digital twin provisioning (Ash)
- Richer memory indexing (semantic, not just keyword)
- Pack Board web UI
