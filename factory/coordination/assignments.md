# Harkonnen-Labs — Active Work Assignments

> All three AIs read this before starting work.
> Claim a task by updating the In Progress table. Release it when done.
> When in doubt, check before touching — you are not the only one here.
>
> API shortcut (once serve is up):
>   POST /api/coordination/claim   { "agent": "...", "task": "...", "files": [...] }
>   POST /api/coordination/release { "agent": "..." }
>   GET  /api/coordination/assignments

---

| AI     | Task                                      | Owns (files/areas)            | Started     |
|--------|-------------------------------------------|-------------------------------|-------------|
| Codex  | cargo check / artifact-lock contention    | src/, Cargo.toml, Cargo.lock  | session-001 |
| Gemini | Coobie Causal Engine Design / Imp | ui/src/, src/orchestrator.rs (Coobie) | session-001 |
| Claude | —                                         | —                             | —           |

---

## Queue (unclaimed)

- [ ] Add `deep_causality` to Cargo.toml (Codex — requested by Gemini)
- [ ] Wire LLM calls into mason / piper / bramble phases in orchestrator.rs (Claude — after Codex clears compile)
- [/] Review and extend Coobie consolidation logic in orchestrator.rs (Gemini)
- [ ] factory/memory/ seed doc quality pass (Gemini)
- [x] UI: verify Pack Board renders with live API data (Gemini)
- [ ] Add ash, flint, sable icons once nano banana delivers them (any)
- [ ] factory-up-linux.sh — test full bring-up end-to-end (Codex)

---

## Completed This Session

| AI     | Task                                      | Notes                          |
|--------|-------------------------------------------|--------------------------------|
| Claude | src/api.rs — /api/coordination/claim + /release + /assignments | AssignmentsState backed by factory/coordination/assignments.json; 409 on file conflict |
| Claude | factory/coordination/assignments.md — this file | AI coordination protocol, file ownership map, queue |
| Claude | src/llm.rs — LlmProvider trait + Anthropic + Gemini clients | reqwest 0.12, async-trait |
| Claude | scout_intake — real LLM call with rule-based fallback | tries claude provider first |
| Claude | vite.config.js — /api proxy to localhost:3000 | dev server now proxies to axum |
| Claude | App.css — cleared Vite boilerplate | global tokens stay in index.css |
| Claude | scripts/ — port checks, Docker daemon liveness, npm error surfacing, .env injection fix, AnythingLLM readiness wait | factory-up-linux.sh + windows.ps1 + bootstrap-local-stack.sh |
| Claude | Coobie architecture decision | SQLite episodic/causal layers, TypeDB deferred |

---

## Coordination Rules

1. **Check this file before starting any task.** If a file you need is owned, wait or coordinate.
2. **One owner per file at a time.** `src/api.rs` and `src/orchestrator.rs` are high-contention — announce before touching.
3. **Cargo.toml changes require all-clear from Codex.** Dependency additions affect the full compile.
4. **factory/memory/ is append-safe** — multiple AIs can add new `.md` files simultaneously; do not edit existing ones without claiming.
5. **Never run `cargo check` or `cargo build` while Codex has the lock.** Check this file first.
6. **Update this file immediately** when you claim or release a task — don't batch updates.

---

## File Ownership Map (stable assignments)

| Area                        | Primary Owner | Notes                              |
|-----------------------------|---------------|------------------------------------|
| src/llm.rs                  | Claude        | LLM provider abstractions          |
| src/orchestrator.rs         | Claude        | agent phase wiring                 |
| src/api.rs                  | Claude        | REST endpoints                     |
| src/db.rs                   | Codex         | schema, migrations                 |
| src/agents.rs               | Codex         | profile loading, execution structs |
| src/setup.rs                | Codex         | config parsing                     |
| src/cli.rs                  | Codex         | CLI surface                        |
| src/memory.rs               | Gemini        | Coobie file-backed index           |
| factory/memory/             | Gemini        | seed docs, memory entries          |
| factory/agents/profiles/    | Gemini        | agent YAML profiles                |
| ui/src/                     | Gemini        | React Pack Board                   |
| scripts/                    | Codex         | bring-up scripts                   |
| Cargo.toml / Cargo.lock     | Codex         | all dep changes go through Codex   |
