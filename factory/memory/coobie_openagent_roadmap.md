---
tags: [coobie, roadmap, openagent, worker-harness, planning, universal, cross-project, factory]
summary: Coobie roadmap for borrowing OpenAgent-style worker-harness ideas without surrendering Harkonnen truth
---
# Coobie OpenAgent Roadmap

Purpose: borrow the strongest orchestration mechanics from OpenAgent while keeping Harkonnen as the outer factory governor.

Priority items:
- Treat any OpenAgent-like system as an inner worker harness for exploration, planning, execution, and visible verification only.
- Keep hidden scenarios, policy, causal memory, digital-twin truth, and release acceptance outside the worker harness.
- Implemented foundation: a retriever-themed bounded forge contract now emits `retriever_task_packet`, `trail_review_chain`, `retriever_dispatch`, and `trail-state` artifacts for any spec that declares a worker harness.
- Implemented executor layer: Harkonnen now runs a real bounded retriever forge executor that consumes the packet, writes retriever execution evidence, and updates trail-state continuity.
- Borrowed from claw-code: add pre/post forge command hooks with Keeper-style allow/deny decisions and Coobie-readable payloads before relying only on final forge summaries.
- Borrowed from claw-code: keep hierarchical repo-local instruction discovery in mind for `.harkonnen/` context layering, but treat operator slash commands as lower priority than hook evidence and continuity.
- Next build layer: let Coobie score returned forge artifacts and hook evidence to critique or route future forge attempts.
- Preserve worker continuity in a separate state file so resumability helps long jobs without replacing Harkonnen's run DB, blackboard, or artifact truth.
- Implemented: hierarchical repo-local context and skill bundles under `.harkonnen/` now feed Coobie preflight and the retriever forge through `retriever_context_bundle` artifacts.
- Implemented foundation: `trail_drift_guard` and `trail_drift_check` now fingerprint code-under-test and repo-local context paths so the forge fails closed when the staged workspace drifts after planning.
- Next layer: evolve the drift guard into a richer edit substrate with line- or region-level anchors for future code-modifying worker harnesses.

Expected outcome:
- Harkonnen can use an OpenAgent-style forge for disciplined planning and execution while Coobie, Sable, Keeper, Ash, and Flint still decide what is true, allowed, hidden, and accepted.
