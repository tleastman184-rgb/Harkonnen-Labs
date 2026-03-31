---
tags: [setup, configuration, providers, environments, windows, linux]
summary: How to switch between setup environments and what each provides
---

# Setup Guide

## Setup Files

| File                      | Environment            | Providers           | Docker? |
|---------------------------|------------------------|---------------------|---------|
| harkonnen.toml            | Home Linux (default)   | Claude+Gemini+Codex | Yes     |
| setups/home-linux.toml    | Home Linux (explicit)  | Claude+Gemini+Codex | Yes     |
| setups/work-windows.toml  | Work Windows           | Claude only         | No      |
| setups/ci.toml            | CI / GitHub Actions    | Claude Haiku only   | No      |

## Switching Setups

Linux/Mac:
    export HARKONNEN_SETUP=work-windows
    cargo run -- setup check

Windows (PowerShell):
    $env:HARKONNEN_SETUP = "work-windows"
    cargo run -- setup check

## Provider Routing

Agents with `provider: default` use whichever provider is named in `[providers] default`.
Agents with `provider: claude` always use Claude regardless of setup.
Change the default by editing `[providers] default = "gemini"` in the active TOML.

## Work Windows — No Docker Needed

AnythingLLM and OpenClaw are replaced by:
- @modelcontextprotocol/server-memory  (Coobie's memory/RAG)
- @modelcontextprotocol/server-filesystem (file access for all agents)
- Claude's 200K context (large-context retrieval without chunking)
- Claude Code's native MCP support

Prerequisites: Node.js, Rust/cargo, ANTHROPIC_API_KEY.

Bootstrap:
    .\scripts\bootstrap-windows.ps1
    # edit .env: set ANTHROPIC_API_KEY and HARKONNEN_SETUP=work-windows
    cargo run -- memory init
    cargo run -- setup check
    cargo run -- serve --port 3057

## Multi-AI Coordination

When working in a shared environment (e.g. Gemini + Codex + Claude):
1. **Autorun the API**: `cargo run -- serve --port 3057`
2. **Claim Files**: Post active tasks to `POST /api/coordination/claim`.
3. **Respect Ownership**: Never modify files claimed by another node in `assignments.json`.
