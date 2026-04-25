# Calvin Archive Bootstrap

This directory is the first bootstrap of the Calvin Archive as a first-class subsystem.

It is intentionally narrow:

- the TypeDB schema is a minimal continuity schema focused on Coobie's identity kernel
- the first seed dataset captures what does **not** change about Coobie
- the human-readable JSON projection mirrors the canonical bootstrap in a form that is easy to inspect in Git

## Files

- `typedb/schema.tql`
  Minimal Phase 6 TypeDB 3.x schema — soul, agent-self, belief, trait, and
  basic relations. Active for Phase 6 development.
- `typedb/schema_phase8.tql`
  Full Phase 8 Calvin Archive schema. Replaces schema.tql when Phase 8 opens.
  Covers all six chambers (Mythos, Episteme, Ethos, Pathos, Logos, Praxis),
  the Meta-Governor integration machinery (integration-candidate,
  integration-policy, quarantine-entry), continuity snapshots, and factory
  wiring (run-record, spec-context, artifact, summary-view).
- `typedb/coobie_kernel_seed.tql`
  Seed data for Coobie's invariant soul kernel. Compatible with schema.tql
  (Phase 6) and schema_phase8.tql (Phase 8 — same entities, richer schema).
- `projections/coobie_identity_kernel.json`
  Inspectable projection of the same seed model for humans and tooling.
- `coobie_soul_guide.md`
  Plain-English explanation of what Coobie preserves and what changes are allowed.

## Generate / Refresh

Run:

```sh
cargo run -- soul bootstrap
```

That command refreshes the schema, seed, and JSON projection from the typed Rust source in `src/calvin_archive/`.

## Boot A Local TypeDB

A Docker-based bootstrap scaffold is available at:

```sh
./scripts/bootstrap-calvin-archive-typedb.sh
```

That script writes a local TypeDB compose stack and helper wrappers in the same style as the Coobie memory stack bootstrap.

## Why This Starts With Coobie

Coobie is the right first persisted self because she already spans:

- memory retrieval
- causal reasoning
- cross-run continuity
- operator-reviewed consolidation

If the continuity model cannot make Coobie legible, it is too weak for the rest of the pack.
