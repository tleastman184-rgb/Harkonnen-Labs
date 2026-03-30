---
tags: [system, overview, factory, harkonnen-labs]
summary: Harkonnen Labs system overview, purpose, and architecture
---

# Harkonnen Labs — System Context

## What It Is

A local-first, spec-driven AI software factory. Humans define intent in YAML specs.
A pack of specialist agents (Scout, Mason, Piper, Bramble, Sable, Ash, Flint, Coobie, Keeper)
perform the implementation work inside a constrained, observable factory.

## Core Goals

- Spec-first: precise intent precedes all implementation
- Outcome-based: acceptance is driven by behavioral evidence, not code review
- Local-first: all state (specs, runs, artifacts, memory) lives on disk
- Role separation: each agent has bounded tools and responsibilities
- Compound learning: each run makes future runs better via Coobie

## Key Directories

- factory/specs/        YAML spec files (the factory's input)
- factory/scenarios/    Hidden behavioral scenarios (Sable only)
- factory/workspaces/   Per-run isolated workspaces
- factory/artifacts/    Output bundles after each run
- factory/memory/       Coobie's memory store (this directory)
- factory/logs/         Run logs
- products/             Target product codebases
- src/                  Rust source for the factory CLI
- setups/               Named setup TOML files (one per environment)

## CLI Entry Points

    cargo run -- spec validate <file>
    cargo run -- run start <spec> --product <name>
    cargo run -- run status <run-id>
    cargo run -- run report <run-id>
    cargo run -- artifact package <run-id>
    cargo run -- memory init
    cargo run -- memory index
    cargo run -- setup check

## Current Status (MVP)

Spec loading, run creation, workspace isolation, artifact packaging, and memory
indexing are implemented. Agent execution adapters, hidden scenario evaluation,
and digital twin provisioning are planned for the next build layer.
