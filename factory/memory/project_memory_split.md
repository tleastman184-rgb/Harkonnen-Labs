---
tags: [coobie, memory, project-memory, core-memory, workflow]
summary: Separate project-local learnings from Harkonnen core memory; promote only strong cross-project patterns into the core store.
---
Coobie should maintain a two-layer durable memory model.

- Project memory lives with the active repo in `.harkonnen/project-memory/` and should hold repo-specific truths, runtime facts, oracle semantics, and local failure modes.
- Core memory in `factory/memory/` should retain only strong cross-project lessons, universal factory patterns, and durable guardrails.
- Runs should retrieve both layers, but new repo-specific run summaries and lessons should be written back to project memory by default.
