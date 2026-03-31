---
tags: [mcp, tools, servers, integration, filesystem, memory, github, sqlite, brave]
summary: MCP server registry — which servers back which abstract tool aliases
---

# MCP Tools Registry

MCP servers are configured in the active setup TOML under [[mcp.servers]].
Each server declares tool_aliases that match the allowed_tools entries in agent profiles.

## Available Servers

### filesystem
Package: @modelcontextprotocol/server-filesystem
Tool aliases: filesystem_read, workspace_write, artifact_writer
Purpose: Read/write access to ./products, ./factory/workspaces, ./factory/artifacts
Platform: all

### memory
Package: @modelcontextprotocol/server-memory
Tool aliases: memory_store, metadata_query
Purpose: Persistent key-value memory for Coobie. Replaces AnythingLLM on work-windows.
Platform: all
Note: Set MEMORY_FILE_PATH=./factory/memory/store.json for cross-session persistence.

### sqlite
Package: @modelcontextprotocol/server-sqlite
Tool aliases: metadata_query, db_read
Purpose: Agent-level read access to factory/state.db (run metadata, history)
Platform: all

### github
Package: @modelcontextprotocol/server-github
Tool aliases: fetch_docs, github_read
Purpose: Repo search, file reads, issue/PR access for Piper and Scout
Requires: GITHUB_TOKEN env var
Platform: all

### multi-ai-coordination
Package: internal (axum /api/coordination)
Tool aliases: assignments_claim, assignments_release
Purpose: Authorization and file-locking across Gemini, Codex, and Claude.
Platform: all

### deep-causality
Package: @harkonnen-labs/server-deep-causality (internal)
Tool aliases: causality_reason, counterfactual_simulate, context_builder
Purpose: Causal reasoning and intervention discovery for Coobie.
Platform: linux

## Adding a New MCP Server

1. Add [[mcp.servers]] entry to the active setup TOML
2. Create factory/mcp/<name>.yaml with the server's documentation
3. Add tool_aliases that agents can reference in their allowed_tools list
4. Run: cargo run -- setup check  (verifies the command is on PATH)
