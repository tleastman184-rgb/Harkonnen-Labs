---
tags: [run, project-memory, wire-fallback-for, Harkonnen-Labs, completed-with-issues]
summary: Run c4ee92c5-2d93-40ae-bdfc-4cc98ee9e049 for Wire CapacityState::fallback_for into build_provider_with_capacity
source_label: Harkonnen-Labs
source_kind: path
source_path: /media/earthling/Caleb's Files1/Harkonnen-Labs
source_run_id: c4ee92c5-2d93-40ae-bdfc-4cc98ee9e049
source_spec_id: wire-fallback-for
git_branch: main
git_commit: b7317ee56bb58e89d028e7f18c649c6107461b9a
git_remote: https://github.com/durinwinter/Harkonnen-Labs.git
evidence_run_ids: [c4ee92c5-2d93-40ae-bdfc-4cc98ee9e049]
stale_when: [git commit or branch changes for the target repo, hidden scenario oracle, dataset, or runtime assumptions change]
observed_paths: [src/llm.rs, src/capacity.rs]
code_under_test_paths: [src/llm.rs, src/capacity.rs]
---

Spec: wire-fallback-for
Product: Harkonnen-Labs
Visible validation passed: true
Hidden scenarios passed: false
Recommended steps: Read Coobie's preflight briefing and required guardrails, Retrieve prior patterns with Coobie, Stage an isolated product workspace, Run visible validation before scenario work, Evaluate hidden scenarios with Sable, Package evidence for human review

Project memory root: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-memory

Top memory hits:
[project context] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-context.md] # Project Context

- Project: Harkonnen-Labs
- Source path: /media/earthling/Caleb's Files1/Harkonnen-Labs
- Project memory root: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-memory
- Evidence root: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/evidence
- Project scan: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-scan.md
- Project manifest: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-manifest.json
- Resume packet: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/resume-packet.md
- Strategy register: /media/ear

[project scan] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-scan.md] # Project Scan

- Generated at: 2026-04-06T17:48:03.488870464+00:00
- Project: Harkonnen-Labs
- Source kind: path
- Source path: /media/earthling/Caleb's Files1/Harkonnen-Labs
- Project memory root: /media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-memory
- Git: branch=main commit=b7317ee56bb58e89d028e7f18c649c6107461b9a remote=https://github.com/durinwinter/Harkonnen-Labs.git clean=true

## Detected Files
- Cargo.toml
- README.md

## Detected Directories
- src
- ui
- scripts

## Likely Commands
- cargo check
- cargo test -q

## Runtime Hints
- Repo appears to contain both backend and UI surfaces.
- Repo already contains Harkonnen-local continuity files.

[strategy register] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/strategy-register.md] # Strategy Register

- Project: Harkonnen-Labs
- Entries: 2

- [d63dd0b2-1179-4b03-b1a4-be371fce5b47:hidden_scenarios-281ce3b5-9e46-43cd-a9aa-8f1c3de193f8] phase=hidden_scenarios agent=sable strategy=sable - pack is workin
thasrealnotgrate
Evaluating hidden scenarios failure_constraint=thasnotgrate
thasrealnotgrate
Hidden scenario evaluation finished: 2 scenario(s) reformulation=hidden_scenarios phase failed; preserve surviving structure and change strategy on retry
- [1647810b-cc75-474a-a65a-4da98ada47da:hidden_scenarios-77951de8-3754-4beb-89d4-2f25f67edb2b] phase=hidden_scenarios agent=sable strategy=sable - pack is workin
thasrealnotgrate
Evaluating hidden scenarios failure_constraint=thasnotgrate
thasrealnotgrate
Hidden scenario evaluation finished: 1 scenario(s) reformulation=hidden_s

[resume packet] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/resume-packet.md] # Resume Packet

- Generated at: 2026-04-06T17:59:58.328299160+00:00
- Project: Harkonnen-Labs
- Current git: branch=main commit=b7317ee56bb58e89d028e7f18c649c6107461b9a remote=https://github.com/durinwinter/Harkonnen-Labs.git clean=false changed_paths=5

## Summary
- Current git: branch=main commit=b7317ee56bb58e89d028e7f18c649c6107461b9a remote=https://github.com/durinwinter/Harkonnen-Labs.git clean=false changed_paths=5
- Project memory entries indexed: 8
- Entries currently at risk: 6
- Entries already marked challenged/superseded: 5
- Risk mix: critical=1 high=4 medium=1 low=0
- Working tree changed paths: actory/memory/index.json, actory/state/dead_ends.json, rc/workspace.rs, .harkonnen/, factory/specs/drafts/wire_fallback_for.yaml
- Working tree is dirty; provenance checks may need 

[memory status] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/memory-status.md] # Memory Status

- lesson-exploration-hidden-scenarios-281ce3b5-9e46-43cd-a9aa-8f1c3de193f8 status=challenged superseded_by=none challenged_by=lesson-exploration-hidden_scenarios-281ce3b5-9e46-43cd-a9aa-8f1c3de193f8, lesson-hidden_scenarios-77951de8-3754-4beb-89d4-2f25f67edb2b, lesson-phase-pattern-hidden-scenarios-sable-failure-claude-anthropic-, lesson-exploration-hidden_scenarios-77951de8-3754-4beb-89d4-2f25f67edb2b
- lesson-exploration-hidden-scenarios-77951de8-3754-4beb-89d4-2f25f67edb2b status=challenged superseded_by=none challenged_by=lesson-exploration-hidden_scenarios-77951de8-3754-4beb-89d4-2f25f67edb2b
- lesson-hidden-scenarios-281ce3b5-9e46-43cd-a9aa-8f1c3de193f8 status=challenged superseded_by=none challenged_by=lesson-exploration-hidden_scenarios-281ce3b5-9e46-43cd-a9aa-8f1c

[stale memory history] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/stale-memory-history.md] # Stale Memory History

- Project: Harkonnen-Labs
- Records retained: 6

## Recent Runs
- - run=1647810b-cc75-474a-a65a-4da98ada47da spec=wire-fallback-for generated=2026-04-06T17:55:50.539448970+00:00 entries=3 satisfied=0 partially_satisfied=3 unresolved=0 resolved_since_previous=0
- - run=d63dd0b2-1179-4b03-b1a4-be371fce5b47 spec=sample-feature generated=2026-04-06T17:52:18.861748801+00:00 entries=0 satisfied=0 partially_satisfied=0 unresolved=0 resolved_since_previous=0
- - run=bd691f00-270a-458c-8a50-ce5d367de31d spec=sample-feature generated=2026-04-06T17:50:17.078926380+00:00 entries=0 satisfied=0 partially_satisfied=0 unresolved=0 resolved_since_previous=0
- - run=69b527c6-80f8-4697-9d7e-aa25d18e6342 spec=sample-feature generated=2026-04-06T17:49:21.079694404+00:00 entries=0 satisf

[project memory] [run-1647810b-cc75-474a-a65a-4da98ada47da] Run 1647810b-cc75-474a-a65a-4da98ada47da for Wire CapacityState::fallback_for into build_provider_with_capacity
Spec: wire-fallback-for
Product: Harkonnen-Labs
Visible validation passed: true
Hidden scenarios passed: false
Recommended steps: Read Coobie's preflight briefing and required guardrails, Retrieve prior patterns with Coobie, Stage an isolated product workspace, Run visible validation before scenario work, Evaluate hidden scenarios with Sable, Package evidence for human review

Project memory root:

[project memory] [lesson-phase-pattern-hidden-scenarios-sable-failure-claude-anthropic] Repeatable failure pattern in hidden_scenarios / sable via claude with anthropic/webapp-testing
Occurrences: 2
Supporting runs: d63dd0b2-1179-4b03-b1a4-be371fce5b47
Provider route: claude
Prompt bundle fingerprint: e8b6470a2c051708
Prompt bundle artifact: agents/sable_prompt_bundle.json
Pinned skills: anthropic/webapp-testing
Required checks: Visible validation must prove the main project still builds or executes in the staged workspace. | The twin narrative must state which external systems

[project memory] [00-project-memory-guide] Repo-local Coobie memory guide for Harkonnen-Labs
# Project Memory Guide

This directory is the durable home for knowledge that should travel with this repo.

Store here:
- domain facts specific to this product
- runtime/API details
- dataset and oracle semantics
- line-specific tuning or commissioning lessons
- accepted mitigations and known failure modes

Do not keep everything in Harkonnen core memory. Promote only durable cross-project patter

[project memory] [run-d63dd0b2-1179-4b03-b1a4-be371fce5b47] Run d63dd0b2-1179-4b03-b1a4-be371fce5b47 for Sample Feature
Spec: sample-feature
Product: Harkonnen-Labs
Visible validation passed: true
Hidden scenarios passed: false
Recommended steps: Read Coobie's preflight briefing and required guardrails, Retrieve prior patterns with Coobie, Stage an isolated product workspace, Run visible validation before scenario work, Evaluate hidden scenarios with Sable, Package evidence for human review

Project memory root: /m

[repo-local context] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/resume-packet.md] - Generated at: 2026-04-06T17:59:58.328299160+00:00

[repo-local context] [/media/earthling/Caleb's Files1/Harkonnen-Labs/.harkonnen/project-scan.md] - Generated at: 2026-04-06T17:48:03.488870464+00:00