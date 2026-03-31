---
tags: [agents, roster, roles, tools, provider]
summary: Full roster of the nine specialist agents with roles, tools, and provider assignments
---

# Agent Roster

Each agent has a bounded role, explicit tool permissions, and a provider assignment.
Trust-critical agents (Scout, Sable, Keeper) pin to Claude.
Implementation agents use the setup's default provider and can be swapped.

| Agent   | Role                | Provider  | Key Allowed Tools                            |
|---------|---------------------|-----------|----------------------------------------------|
| Scout   | Spec retriever      | claude    | filesystem_read, spec_loader, mcp:filesystem |
| Mason   | Build retriever     | default   | workspace_write, filesystem_read, mcp:filesystem |
| Piper   | Tool retriever      | default   | build_runner, filesystem_read, container_runner |
| Bramble | Test retriever      | default   | build_runner, workspace_write, mcp:filesystem |
| Sable   | Scenario retriever  | claude    | scenario_store, twin_runner, report_writer   |
| Ash     | Twin retriever      | default   | container_runner, filesystem_read            |
| Flint   | Artifact retriever  | default   | artifact_writer, filesystem_read             |
| Coobie  | Causal Reasoner     | default   | deep_causality, memory_store, counterfactuals |
| Keeper  | Coordination Auth   | claude    | policy_engine, assignments_auth, secrets     |

## Orchestration Tier

Harkonnen Labs operates with a three-node AI orchestration protocol:
- **Gemini**: Lead UI & Memory Architect (Specializing in Blackboard & Causal Engine).
- **Codex**: Core Engine & Infrastructure (Specializing in Rust, Cargo, and Build systems).
- **Claude**: Logic & Agent Orchestration (Specializing in LLM routing and Phase logic).

## Key Invariants

- Mason cannot read scenario_store (prevents test gaming)
- Sable cannot write implementation code
- Keeper alone holds policy_engine access
- All agents share the labrador personality: loyal, persistent, honest, non-bluffing

## Profile Files

factory/agents/profiles/<name>.yaml — one file per agent
factory/agents/personality/labrador.md — shared personality
