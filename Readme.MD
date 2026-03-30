# Harkonnen Labs

Level 5 AI Software Factory MVP.

## What this is

Harkonnen Labs is a local-first software factory that turns:

- specs
- hidden scenarios
- prior knowledge

into:

- generated software
- validation reports
- artifact bundles

Humans write specs and judge outcomes.
Agents do the implementation work.

## MVP goals

- CLI-first
- Rust core
- SQLite for run metadata
- filesystem-backed specs/scenarios/artifacts
- named agent profiles
- hidden scenario isolation
- sample target product

## Quick start

```bash
cargo run -- spec validate factory/specs/examples/sample_feature.yaml
cargo run -- run start factory/specs/examples/sample_feature.yaml --product sample-app
cargo run -- run status <run-id>
cargo run -- run report <run-id>code