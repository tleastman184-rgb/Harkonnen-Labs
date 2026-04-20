# Harkonnen Labs — Execution Roadmap

**This is the canonical build order from 2026-04-17 forward.**
Phases 1, 4, 4b, 5, v1 (A–D), and 2 are shipped. Phase 3 is the active build target.

---

## Maturity Ladder

This roadmap is the canonical build sequence from today's factory state to true Phase 4 agentic intelligence and beyond.

| Maturity phase | Meaning | Harkonnen status |
| --- | --- | --- |
| Phase 1 — Assisted Intelligence | Copilots, chatbots, drafting help | Already surpassed |
| Phase 2 — Automated Intelligence | Rule-based workflows, permissions, governance | Already surpassed as a standalone destination |
| Phase 3 — Augmented Intelligence | Core agent with proactive suggestions, learning loops, human confidence-building | Current baseline |
| Phase 4 — Agentic Intelligence | Self-directed agents inside explicit guardrails, with structural coordination and self-monitoring | Active destination — Phase v1 closes the remaining gap |

### What still separates Harkonnen from Phase 4 (verified in source, 2026-04-18)

A structured gap analysis identified seven practical gaps. Gap-closure phases A–D addressed five of them. Three structural gaps remain before the system can legitimately claim Tier 4:

| Gap | Gap-closure status |
| --- | --- |
| Enforced authority and guardrail boundaries | **Open** — `check_lease` API exists but is never called pre-write; guardrails are advisory only |
| Live world-state modeling | Partial — twin is a manifest, not provisioned infrastructure (Phase 3) |
| Closed-loop outcome verification | Partial — observation endpoint deferred to Phase E (TypeDB dependency) |
| Structural multi-agent coordination | Mostly closed — blackboard, heartbeat, claim eviction are real |
| Economic and cost awareness | Closed — A1 trace spine + cost events |
| Explicit intent → plan → execution separation | Closed — B, C (OptimizationProgram) |
| External system interfaces | Open — Phase v1 External Integrations track |

### How this roadmap closes that gap

- `.harkonnen/gap-closure-progress.md` tracks strategic bridge work phases A–D (all shipped)
- Phase v1 (below) is the structural gate before the factory can be called Tier 4
- Phase E remains deferred on the TypeDB-backed state graph
- Numbered execution phases (2–7) stay focused on factory capability: real test execution, live twins, memory infrastructure, TypeDB, causal corpus
- Operator Model Activation and External Integrations are parallel product tracks

## Why this order

The factory has a complete foundation: core pipeline, PackChat control plane, layered Coobie memory, causal graph, Pearl hierarchy labeling, multi-hop retrieval, operator-reviewed consolidation Workbench, agent trace spine, optimization programs, and adversarial metric attacks. The remaining gaps before Tier 4 are concrete and bounded: guardrails are advisory instead of enforced, the memory invalidation persistence layer is incomplete, and there are no outbound integrations. Phase v1 closes those gaps. After that, Phase 2 makes Bramble's validation score meaningful and Phase 3 grounds Sable's twin.

Benchmarking remains a parallel track. Each phase ships with at least one measurable gate.
The benchmark philosophy should remain explicitly agentic-engineering shaped:
measure how quickly and safely software moves through the delivery system, not
just how quickly code is emitted. That means coordination compression, downstream
validation speed, and time-to-root-cause matter alongside code-level success.

---

## Phase v1 — Tier 4 Finalization

**This is the active build target.** Closes the remaining structural gaps that prevent Harkonnen from being called a genuine Tier 4 agentic workflow. Phases 2 and 3 begin after this phase is done.

### v1-A — Guardrail Enforcement (hard blocker for Tier 4)

**Why it's a blocker:** Tier 4 requires agents to operate *inside* explicit guardrails, not just record them. Currently `check_lease` exists in `src/api.rs` but is never called from `src/orchestrator.rs`. Every Mason file write bypasses the lease system. Decision records are written *after* the fact, not enforced *before* the act.

**What to build:**

- Call `POST /api/coordination/check-lease` inside `mason_generate_and_apply_edits` before writing any file — pass `resource_kind: "workspace"`, the staged path prefix, and the run's guardrails from the Coobie briefing. If the response is denied or returns violations, return an error rather than proceeding and write a decision record explaining why the write was blocked.
- Wire the same check in the Mason plan generation path: claim `resource_kind: "workspace"` at plan start with `ttl_secs` derived from the spec's time budget
- Add at least three more `record_decision` call sites: Mason plan selection, Scout optimization program derivation, Sable attack generation. Currently only Coobie critique and consolidation promotion are wired.
- Add `GET /api/runs/:id/decisions` to the Pack Board run detail drawer so operators can inspect the decision audit trail per run

**Done when:** A Mason edit attempt against a path that has no active workspace lease is blocked at the orchestrator level, a decision record is written explaining the block, and the Pack Board surfaces the decision log per run.

---

### v1-B — Memory Invalidation Persistence (Phase 4b completion)

**Why:** The ROADMAP header previously claimed Phase 4b was shipped. A source audit found that invalidation is computed at query time only — there is no `memory_updates` table, no `invalidated_by` column on memory records, and no `GET /api/memory/updates` endpoint. The StreamingQA adapter cannot score belief-update accuracy against a persistence layer that does not exist.

**What to build:**

- `memory_updates` table in `src/db.rs`: `(update_id, old_memory_id, new_memory_id, reason, created_at)`
- `invalidated_by: Option<String>` on the memory record schema (references `update_id`)
- Coobie ingest pipeline: before writing a new memory entry, check for semantic near-duplicates with conflicting claims via cosine similarity on the embedding store. If found above threshold, write a supersession record and set `invalidated_by` on the old entry.
- `GET /api/memory/updates` endpoint returning supersession history
- Memory Board UI panel: distinguish invalidated entries from current entries; allow operator to confirm or reject a supersession

**Done when:** Ingesting a new fact that contradicts an older one persists a supersession record, the old entry is flagged in the DB, and `GET /api/memory/updates` returns the history. StreamingQA can then score belief-update accuracy against real persistence.

---

### v1-C — FailureKind Classification

**Why:** Mason's fix loop handles all failures identically. A wrong-answer failure (test ran, output was wrong) requires a different fix prompt than a compile error (code never ran). The ROADMAP Phase 2 spec calls this out as `FailureKind::WrongAnswer` but it was never implemented as an enum variant.

**What to build:**

- `FailureKind` enum in `src/models.rs`: `CompileError`, `TestFailure`, `WrongAnswer`, `Timeout`, `Unknown`
- Parser in the fix loop that classifies stdout/stderr: detect "expected … got …", "FAILED", "assertion failed", exit code patterns
- `WrongAnswer` variant triggers a distinct Mason prompt that includes the expected vs actual diff rather than the raw compiler error
- `failure_kind` field on `ValidationSummary` so Coobie can pattern-match on failure type in its causal records

**Done when:** A run with a wrong-answer test failure shows `failure_kind: WrongAnswer` in the run summary and Mason's fix attempt uses the diff-focused prompt.

---

### v1-D — Operator Model Minimum Viable

**Why:** The five-layer interview tables exist in the DB but have no logic. Scout's intent generation and Coobie's preflight have no connection to operator context. Without this, every new spec starts from scratch regardless of how well Coobie knows the operator's patterns.

**What to build (two-layer MVP, not the full five-layer spec):**

- PackChat `interview` command: initiates a two-layer intake (operating rhythms + recurring decisions) with checkpoint approval after each layer
- `commissioning-brief.json` artifact generated from the approved layers: contains operator's primary work patterns, preferred tools, recurring decisions, and risk tolerances
- Scout draft integration: when a `commissioning-brief.json` exists for the operator, Scout includes its top-3 patterns in the intent package prompt
- Coobie preflight integration: operator's stated risk tolerances contribute to `required_checks` and guardrail text
- Update the `operator_model_sessions` and `operator_model_layer_checkpoints` tables (schema exists; no logic is wired)

**Done when:** An operator who has completed the two-layer interview sees their stated patterns reflected in Scout's intent packages and Coobie's required checks on subsequent runs.

---

### v1 Benchmark / product gate

- Decision audit log surfaced in Pack Board per run
- Memory supersession events returned by `GET /api/memory/updates`
- At least one run showing `failure_kind: WrongAnswer` in the validation summary
- At least one run where Scout's intent package references operator model context

---

## Phase 2 — Bramble Real Test Execution

**Unlocks:** Coobie's `validation_passed` score becomes meaningful.
`TEST_BLIND_SPOT` and `PACK_BREAKDOWN` causal signals currently score against stubs.

**What to build:**

- `bramble_run_tests` in orchestrator — reads `spec.test_commands` (same detection logic as Piper) and executes them in the staged workspace
- Stdout/stderr streamed as `LiveEvent::BuildOutput` on the broadcast channel (already exists — Bramble just needs to use it)
- `ValidationSummary` populated from real exit codes and parsed test output, not from scenario results or stubs
- Bramble's phase attribution records `validation_passed: true/false` from actual runs
- Feed result back as `test_coverage_score` into the Coobie episode at ingest time
- **Mason online-judge feedback loop** — `FailureKind::WrongAnswer` (wired in Phase v1-C) feeds into Mason's fix loop with a diff-focused prompt. Phase 2 formalises the loop end-to-end: parse stdout diff output from competitive programming judges as a first-class failure signal.
- **LiveCodeBench adapter** — wrapper command that pulls recent problems, runs Mason/Piper, and emits pass/fail per problem into the benchmark runner.
- **Aider Polyglot adapter** — maps Aider's multi-language benchmark format to Harkonnen specs; no structural changes needed.

**Benchmark gate:**

- `local_regression` stays green on every merge
- the code loop should be runnable through the emerging `SWE-bench Verified` adapter, even if scores are unpublished
- `LiveCodeBench` adapter wired and producing artifacts
- `Aider Polyglot` adapter wired for a direct open-source comparison line

**Done when:** A spec with `test_commands` shows real pass/fail in the run report, Coobie's episode scores reflect actual test execution, and Mason's fix loop handles wrong-answer failures.

---

## Phase 3 — Ash Real Twin Provisioning

**Unlocks:** Sable's scenario evaluation becomes grounded.
Right now Sable judges against a twin that is a JSON manifest, not running infrastructure.

### 3-A — Models and Spec extension

- Add `TwinFailureMode` enum to `src/models.rs`: `AuthExpiry`, `RateLimit`, `ConnectionRefusal`
- Add `TwinServiceSpec` struct to `src/models.rs`: `name: String`, `image: String`, `port: Option<u16>`, `env: BTreeMap<String, String>`, `failure_mode: Option<TwinFailureMode>`
- Add `twin_services: Vec<TwinServiceSpec>` (serde default empty) to the `Spec` struct in `src/models.rs`
- Add `compose_project: Option<String>` to `TwinEnvironment` so teardown can find the right stack by project name

### 3-B — `ash_provision_twin` in `src/orchestrator.rs`

Replaces the current `build_twin_environment` call in the twin phase.

- If `spec.twin_services` is empty AND `spec.dependencies` is non-empty, derive `TwinServiceSpec` entries automatically by mapping known names (postgres, redis, mysql, mongo, rabbitmq, kafka, minio) to well-known images and default ports; unknown deps get `busybox:latest` as a placeholder
- Generate `docker-compose.yml` in `run_dir/` — one service per spec, applying `failure_mode` env vars: `AuthExpiry → MOCK_AUTH_EXPIRED=true`, `RateLimit → MOCK_RATE_LIMIT=10`, `ConnectionRefusal` → service omitted from compose entirely
- Run `docker compose -p <project> -f <compose_file> up -d` (fall back gracefully to simulated mode if docker is unavailable — check `setup.machine.fingerprint.docker`)
- Poll readiness: retry `docker compose -p <project> ps` up to 10 times with 1-second intervals; mark each service "running" or "failed"
- Get port bindings: `docker compose -p <project> port <service> <internal_port>` for each service; write `twin_env.json` to run_dir: `{ "services": { "postgres": "127.0.0.1:54321", ... } }`
- Build and return a `TwinEnvironment` where each service's `status` is `"running"`, `"failed"`, or `"simulated"` (if docker unavailable)

### 3-C — `ash_teardown_twin` in `src/orchestrator.rs`

- Called after the Sable/hidden-scenarios phase completes (currently line ~3200 in orchestrator)
- Runs `docker compose -p <project> down --remove-orphans`
- Logs teardown result to the run event log; does not fail the run if teardown errors

### 3-D — Wire provisioning into the run lifecycle

- Replace `let twin = self.build_twin_environment(run_id, spec_obj);` with `let (twin, compose_project) = self.ash_provision_twin(run_id, spec_obj, &run_dir).await?;`
- Store `compose_project` in the blackboard so teardown can find it
- Add teardown call after the hidden-scenarios phase completes
- Pass `twin_env.json` path to Sable's scenario evaluation context so scenarios can reference real port bindings

### 3-E — Update `twin_fidelity_score` in `src/coobie.rs`

- Change the fidelity computation (currently line ~1221) to count services where `status == "running"` rather than just counting total services
- Formula: `running_count / total_declared` — gracefully handles zero total (returns 0.1 as before)

### 3-F — Flint documentation phase

- After `self.package_artifacts(run_id)` in the Flint phase, call a new `flint_generate_docs` method
- `flint_generate_docs` reads the spec and Mason's implementation artifacts from the run dir, calls the Flint LLM agent to generate a `README.md` and optionally an `API.md`
- Writes output to `artifacts/docs/<run_id>/README.md` and `artifacts/docs/<run_id>/API.md`
- Adds `docs/README.md` to `blackboard.artifact_refs`
- Required for DevBench — must land in Phase 3

### 3-G — `src/spec_adherence.rs` — LLM-as-judge benchmark

New builtin benchmark module (follows the same pattern as `cladder.rs`).

- Loads a JSONL file where each line is `{ "run_id": "...", "spec_path": "...", "output_path": "..." }`, OR if no dataset is provided, queries the local SQLite DB for the last N completed runs
- For each entry: reads the spec's `acceptance_criteria` list and Mason's primary output artifact, asks an LLM judge to score each criterion as met/partial/unmet
- Metrics: `completeness` (fraction of criteria met or partial), `precision` (fraction fully met)
- Env: `SPEC_ADHERENCE_DATASET`, `SPEC_ADHERENCE_LIMIT`, `SPEC_ADHERENCE_OUTPUT`, `SPEC_ADHERENCE_MIN_COMPLETENESS`
- Builtin name: `"spec_adherence"`
- Also supports a `without_scout` mode to measure what Scout's formalization step contributes

### 3-H — `src/scenario_delta.rs` — Hidden Scenario Delta benchmark

New builtin benchmark module — Harkonnen-native, no external dataset.

- Queries `coobie_episode_scores` in the local SQLite DB for runs where both `validation_passed` and `scenario_passed` are recorded
- Computes: `visible_pass_rate` (fraction where `validation_passed = 1`), `hidden_pass_rate` (fraction where `scenario_passed = 1`), `delta = visible_pass_rate - hidden_pass_rate`
- A large positive delta means Bramble passes things that Sable catches — proves the hidden scenario value
- Writes `scenario_delta_report.md` and `scenario_delta_summary.json` to artifact dir
- Builtin name: `"scenario_delta"`
- Env: `SCENARIO_DELTA_LIMIT` (max runs to include), `SCENARIO_DELTA_OUTPUT`

### 3-I — `suites.yaml` entries

- `harkonnen_spec_adherence` — Spec Adherence Rate (harkonnen-native, builtin: `spec_adherence`)
- `harkonnen_scenario_delta` — Hidden Scenario Delta (harkonnen-native, builtin: `scenario_delta`)
- `harkonnen_twin_fidelity` — Twin Fidelity (harkonnen-native: queries last N runs for `twin_fidelity_score` distribution, builtin: `scenario_delta` or new `twin_fidelity` builtin)

**Benchmark gate:**

- At least one run with `twin_fidelity_score > 0.5` from real Docker containers
- `spec_adherence` first run published — completeness and precision against local run corpus
- `scenario_delta` first run published — visible vs hidden pass rate gap across recent runs
- `DevBench` adapter wired (script-based, not builtin — deferred to after Flint docs land)

**Done when:** A spec with `twin_services` actually starts Docker containers; `twin_env.json` contains real port bindings; Sable receives those bindings; `twin_fidelity_score` reflects real container health; Flint produces a doc artifact per run; and `spec_adherence` and `scenario_delta` have first-run baselines.

---

## Phase 4b — Memory Invalidation and Fact-Update Tracking

**Status: Partially shipped.** Query-time invalidation reasons exist and are surfaced in retrieval hits. The persistence layer (supersession records, `memory_updates` table, `GET /api/memory/updates`) is being completed in Phase v1-B above. This entry is the benchmark and maintenance reference point once v1-B lands.

**What was built in Phase 1 (query-time only):**

- `invalidation_reasons` field on `MemoryRetrievalHit` — computed at retrieval time from `superseded_by` / `challenged_by` provenance fields
- `memory_invalidation_reasons()` helper in orchestrator surfaces reasons per hit

**What v1-B completes (persistence layer):**

- `memory_updates` table in SQLite: `(update_id, old_memory_id, new_memory_id, reason, created_at)`
- `invalidated_by` on memory records, set at ingest time when a near-duplicate conflict is detected
- `GET /api/memory/updates` endpoint
- Memory Board UI panel: invalidated entries distinguished from current entries
- **StreamingQA native adapter** — streams fact-update events to Coobie's memory, then queries whether the updated belief is correctly recalled. Scores belief-update accuracy separately from static recall.

**Benchmark gate:**

- `StreamingQA` first run published — belief-update accuracy, no competitor publishes this
- re-run `LongMemEval` to confirm invalidation tracking does not regress static recall

**Done when:** Ingesting a new fact that contradicts an older one marks the old fact as invalidated in the DB, the operator can review the supersession, and StreamingQA has a baseline score.

---

## Phase 5b — Memory Infrastructure (Qdrant + OCR)

**Unlocks:** Semantic recall at scale and document ingest completeness. The SQLite vector store is sufficient for current run volume, but it becomes the bottleneck as the memory corpus grows.

**What to build:**

- **Qdrant integration** — add `src/coobie/qdrant.rs` implementing the semantic index over extracted text and memory summaries. Payload metadata: `org`, `role`, `product`, `spec_id`, `run_id`, `agent`, `memory_type`, `tags`, `created_at`. Qdrant replaces the SQLite vector store for long-term semantic memory (keep SQLite as the short-term and episodic store). Bootstrap script at `scripts/bootstrap-coobie-memory-stack.sh` already exists.
- **OCR pipeline** — add Tesseract-backed OCR for scanned PDFs and images. Current extractors handle text-forward formats but cannot read scanned documents. Wire through the existing `memory ingest` path: detect image-only PDFs, invoke `tesseract`, write extracted text sidecar alongside the imported asset.
- **Memory module refactor** — split the growing `src/memory.rs` into the module tree described in COOBIE_SPEC: `src/memory/mod.rs`, `working.rs`, `episodic.rs`, `semantic.rs`, `causal.rs`, `consolidation.rs`, `blackboard.rs`, `retrieval.rs`, `extraction.rs`. No behavior change; this is a maintainability gate before the codebase grows further.

**Benchmark gate:**

- re-run `FRAMES` after Qdrant lands to confirm multi-hop recall improves over the SQLite vector baseline
- `LongMemEval` and `LoCoMo` re-run to confirm semantic recall quality does not regress

**Done when:** Qdrant is serving semantic queries for long-term memory, OCR-scanned PDFs can be ingested, and `src/memory.rs` is split into the COOBIE_SPEC module tree.

---

## Phase 6 — TypeDB Semantic Layer (Layer C)

**Unlocks:** Typed causal queries that vector similarity cannot answer. "Find all runs where TWIN_GAP caused a failure that was fixed by an intervention that held for ≥ 3 runs" requires a graph, not a similarity score.

TypeDB 3.x changes the implementation assumptions: the old JVM burden objection is gone because TypeDB's core is now Rust. It is still an external service with real operational cost, so it stays later in the sequence and should not replace SQLite as the hot path. When this phase opens, use the Rust-based TypeDB 3.x line in a container-first deployment and avoid the legacy Java server/distribution entirely.

**What to build:**

- TypeDB 3.x instance configured in the home-linux setup TOML
- `src/coobie/semantic.rs` implementing the `SemanticMemory` trait from COOBIE_SPEC
- Rust-facing TypeDB adapter using the official TypeDB 3.x driver behind the `SemanticMemory` abstraction
- TypeDB schema from COOBIE_SPEC: entities (agent, goal, episode, observation, action, outcome, artifact, lesson, failure-mode, causal-link), relations as specified
- TypeDB 3.x function-backed semantic reasoning; do not design around legacy rules-engine assumptions
- Write-back: after Phase 5 consolidation approval, promoted lessons and causal links written to TypeDB as well as the file store
- Query surface: `POST /api/coobie/query` routes natural-language causal questions through Coobie's retrieval chain
- Coobie's briefing builder calls TypeDB for cross-run pattern queries before preflight
- **GAIA Level 3 adapter** — maps GAIA's multi-step tool-use tasks to Harkonnen's factory run format; routes sub-tasks to the appropriate Labrador rather than a single generalist. Requires the TypeDB query surface to be live.
- **AgentBench adapters** — OS, database, and web environments, each mapped to a Labrador role.

**Benchmark gate:**

- cross-run causal-query benchmarks comparing SQL aggregate recall versus TypeDB-backed semantic recall
- `GAIA Level 3` first run published
- `AgentBench` first runs across OS, DB, and web environments

**Done when:** You can ask Coobie "what caused the last three failures on this spec" and get an answer from a typed graph; GAIA Level 3 and AgentBench adapters wired and producing artifacts.

---

## Phase 7 — Causal Attribution Corpus and E-CARE

**Unlocks:** The strongest publishable internal benchmark claims. The causal attribution corpus and E-CARE adapter are both spec'd in Phase 5 but can be built incrementally and do not depend on TypeDB.

**What to build:**

- **Causal attribution accuracy corpus** — 30–50 labeled runs with seeded failures (wrong API version, missing env var, breaking schema change, etc.). Each entry has a spec, a seeded failure, a ground-truth cause label, and the Coobie `diagnose` output. Score top-1 and top-3 accuracy. Start with 10 entries for a first baseline. Lives in `factory/benchmarks/causal-attribution/`.
- **E-CARE native adapter** — maps Coobie's `diagnose` output to E-CARE's evaluation format and scores whether generated causal explanations are judged natural-language coherent. Run after consolidation so promoted lessons can inform subsequent diagnose output.
- Publish before/after comparisons for causal attribution accuracy: pre-Phase 4 (pure semantic recall) versus post-Phase 4 (causal graph-augmented).

**Benchmark gate:**

- `E-CARE` first run published — causal explanation coherence score
- `causal attribution accuracy` first run published — top-1 / top-3 vs semantic-only baseline

**Done when:** The corpus has at least 30 labeled entries, the causal attribution accuracy benchmark has a published run, and E-CARE has a published score.

---

## Phase 8 — The Calvin Archive And Governed Integration

**Unlocks:** A persisted intelligence layer that does not merely remember, but
decides what becomes part of itself. This is the phase where Harkonnen moves
from identity continuity as a typed graph to identity continuity as a governed
integration process.

The design for this phase — including the formal metrics and the three-tier data
stack — is specified in [the-soul-of-ai/08-Identity-Continuity.md](the-soul-of-ai/08-Identity-Continuity.md)
and the integration-governance design in [the-soul-of-ai/07-Governed-Integration.md](the-soul-of-ai/07-Governed-Integration.md).

**What to build:**

**Storage layer (three-tier):**

- **TimescaleDB hypertable bootstrap** — episodic behavioral telemetry store for agent events, drift samples, stress accumulation, and SSA snapshots. Hypertable compression policy (7-day chunks, 30-day retention window). Provides the time-series foundation for D* estimation and stress computation.
- **TypeDB Calvin Archive schema** — typed ontological layer for the six chambers (Mythos, Episteme, Ethos, Pathos, Logos, Praxis), integration candidates, quarantine entries, revision graphs, and causal patterns. Schema spec in MASTER_SPEC Part 5.
- **Materialize streaming SQL views** — real-time `D*` drift monitoring (sliding window over TimescaleDB events via SUBSCRIBE), live Meta-Governor alert views, and SSA tracking views. `D*` and SSA are the two primary continuous signals; Φ and F are computed on-demand.

**Governance and integration:**

- Calvin Archive Meta-Governor with explicit `accept`, `modify`, `reject`, and `quarantine` outcomes for identity-relevant integration events
- File-first soul package projection with `soul.json`, `SOUL.md`, `IDENTITY.md`, `AGENTS.md`, `STYLE.md`, `MEMORY.md`, and `HEARTBEAT.md`, generated from and checked against canonical continuity state
- Integrity-hash verification and heartbeat audits so the projected soul package cannot drift silently away from the Calvin Archive
- Quarantine ledger: unresolved items persist with pending evidence conditions, salience decay, and re-evaluation triggers
- Pattern-level reflection over compressed cross-episode structures so schema revision is distinct from ordinary belief revision
- Stress-estimator computation (backed by TimescaleDB) so recurring unresolved strain triggers governed reflection instead of ad hoc self-rewrite
- Slow-loop integration-policy revision flow, more conservative than ordinary updates and naturally attachable to human endorsement
- Cross-layer hysteresis measurement so rollback quality is judged by residual behavioral drift, not only by restored file contents
- Presence continuity checks so model/provider swaps preserve identity semantics rather than resetting the pack by accident
- Pathology detection for trauma-analog overweighting, denial, fragmentation, and hyper-local overfitting

**Metrics implementation (from chapter 07):**

- **`D*` (Drift Bound)** — `D* = α/γ`, where α is behavioral deviation rate (from episodic log) and γ is recovery rate (from consolidation events). Materialize view watches `D*` continuously; Meta-Governor triggered if session drift exceeds bound.
- **SSA (Semantic Soul Alignment)** — cross-domain weighted action-pattern consistency against Labrador persona goals. Computed per run window and stored as a TimescaleDB event.
- **F (Variational Free Energy)** — KL divergence between agent's generative model and actual observations; high F signals that the agent must seek clarification or update beliefs. Computed on-demand, not streamed.
- **Φ (Integrated Information)** — bipartition-minimized causal integration measure over the Calvin Archive graph. Used to gate Calvin Archive updates: a post-learning drop in Φ triggers quarantine rather than direct integration.

**Benchmark gate:**

- D* (unjustified-drift score) published — continuous via Materialize view
- SSA baseline score published — per-run, stored in TimescaleDB
- healthy quarantine-rate / resolution-rate baseline published
- schema-revision stability benchmark published
- stress / hysteresis recovery benchmark published
- Φ post-learning drop detection wired (quarantine trigger, not yet a published score)

**Done when:** Harkonnen can distinguish accepted, rejected, modified, and quarantined identity changes; the projected soul package is verifiable against canonical continuity state; D* and SSA are instrumented and streaming; reflection can revise schemas without overwriting raw experience; rollback quality is measured through hysteresis rather than assumed; and policy-level revision is slower, more conservative, and explicitly reviewable.

---

## Parallel Product Track — Operator Model Activation

**Unlocks:** Better commissioning, fewer mid-run clarification failures, and a reusable operator context layer that Scout, Coobie, and Keeper can all consume.

**Current state:** DB schema is complete (`operator_model_profiles`, `operator_model_sessions`, `operator_model_layer_checkpoints`, `operator_model_entries`, `operator_model_exports`, `operator_model_update_candidates` tables all exist). No interview logic, no layer progression, no artifact generation, no Scout/Coobie integration is wired. Phase v1-D ships the two-layer MVP. The full five-layer spec follows.

**Full five-layer spec (post-v1):**

- Native PackChat-based elicitation workflow with five fixed layers: operating rhythms, recurring decisions, dependencies, institutional knowledge, friction
- Approval checkpoints after each layer, reusing the existing checkpoint and unblock flow
- Artifact generation for `operating-model.json`, `USER.md`, `SOUL.md`, `HEARTBEAT.md`, `schedule-recommendations.json`, plus a Harkonnen-specific `commissioning-brief.json`
- Scout draft integration so spec generation can use an approved operator model as first-class context
- Coobie preflight integration so operator-model assumptions contribute to `required_checks`, guardrails, and escalation rules

**Current shipped slice:** project-first operator-model resolution now influences Scout draft generation and Coobie preflight guidance. The remaining product work is the checkpoint/export/review loop that turns the interview into durable stamped artifacts with operator approval.

- Review loop after runs: consolidation can propose operator-model updates, which the operator can keep/discard/edit before promotion
- Import/export compatibility with OB1-style operating artifacts, but no direct code dependency on OB1

**Benchmark / product gate:**

- Measurable drop in open checkpoints per run for projects using an approved operator model
- Spec draft quality and spec adherence compared with and without the operator model

**Done when:** A user can complete the five-layer interview with approvals, generate operating artifacts, and see those artifacts materially influence Scout draft quality and Coobie preflight behavior.

---

## Parallel Product Track — External Integrations

**Unlocks:** The factory becomes observable and controllable from outside the Pack Board. Without outbound notifications, every run outcome requires a human to poll the UI. Without inbound triggers, specs must be started manually. Without auth, the API is open to anyone on the network.

This is a usability prerequisite for any team or multi-machine deployment. Most items are small and independent; they do not need to ship as a block.

### EI-1 — API Authentication

**Why first:** The HTTP API is currently unauthenticated. Every other integration that touches the API needs auth to be safe.

- API key authentication middleware in `src/api.rs` — bearer token checked on all non-health routes
- `api_keys` table in SQLite: `(key_id, key_hash, label, created_at, last_used_at, revoked)`
- `POST /api/auth/keys` (create), `GET /api/auth/keys` (list), `DELETE /api/auth/keys/:id` (revoke)
- `GET /health` and the SSE stream remain unauthenticated (monitoring and browser clients)
- CLI flag `--api-key` or env var `HARKONNEN_API_KEY` for local development bypass

### EI-2 — Outbound Webhook Notifications

**Why:** A webhook system is the foundation for all other integrations (Slack, Discord, GitHub). Everything downstream subscribes to the same event stream.

- `webhooks` table: `(webhook_id, url, secret, events: JSON array, created_at, enabled)`
- `POST /api/webhooks`, `GET /api/webhooks`, `DELETE /api/webhooks/:id`
- Events emitted: `run.started`, `run.completed`, `run.failed`, `checkpoint.created`, `checkpoint.resolved`, `metric_attack.detected`, `consolidation.ready`
- Payload: `{ event, run_id, spec_id, timestamp, summary, pack_board_url }`
- HMAC-SHA256 signature on the `X-Harkonnen-Signature` header (same pattern as GitHub webhooks)
- Retry with exponential backoff on 5xx or connection failure (up to 3 attempts)

### EI-3 — Slack Integration

**Why:** Operators spend time in Slack. Run outcomes, checkpoints, and Coobie insights need to surface where the operator already is, not require switching to the Pack Board.

**Outbound (Slack notifies operator):**

- Rich block-kit messages on `run.completed`: summary card with pass/fail, agent trace count, cost, decision count, link to Pack Board
- Checkpoint alert with inline Approve / Reject buttons that call back to the Harkonnen API
- `metric_attack.detected` alert: which metric was attacked, which exploit fired, suggested mitigation
- `run.failed` with Coobie's top causal diagnosis (from the latest `diagnose` output)

**Inbound (operator controls factory from Slack):**

- Slash command `/harkonnen run <spec-id>` — triggers a run, responds with run ID and Pack Board link
- `/harkonnen status <run-id>` — returns current phase and latest event
- `/harkonnen ask <question>` — routes to Coobie's `dispatch_message` as a PackChat message
- `/harkonnen checkpoint approve <id>` / `reject <id>` — resolves checkpoints without opening the browser

**Config:** Slack app credentials stored in setup TOML under `[integrations.slack]`. Webhook URL and bot token. No hardcoded values.

### EI-4 — Discord Integration

**Why:** Discord is common in solo operator and small-team contexts (and is where local AI communities live). The surface area is simpler than Slack and the bot API is lower-friction to self-host.

**Outbound:**

- Webhook embeds for `run.completed`, `checkpoint.created`, `run.failed` — same content as Slack but using Discord embed format (color-coded by outcome)
- Thread-per-run option: create a Discord thread for the run and post phase updates as the run progresses

**Inbound (bot commands in a designated channel):**

- `!run <spec-id>` — triggers run
- `!status <run-id>` — current phase and last event
- `!approve <checkpoint-id>` / `!reject <checkpoint-id>`
- `!ask <question>` — routes to Coobie

**Config:** `[integrations.discord]` in setup TOML. Bot token and guild/channel IDs.

### EI-5 — GitHub Integration

**Why:** Mason already creates branches (`mason/<spec-id>-<run-id>`). The natural next step is auto-creating a PR from that branch, posting results as a PR comment, and allowing a GitHub webhook to trigger a spec run on push or PR open.

**Outbound:**

- After a run completes with Mason edits applied: optionally create a PR from the Mason branch using the GitHub API. PR body includes the spec title, run ID, decision log summary, Coobie critique outcome, and Pack Board link.
- Post a run summary as a PR comment when a run is triggered by a PR webhook (see inbound). Comment includes pass/fail, cost, and the top advisory concern from Coobie.

**Inbound:**

- `POST /api/integrations/github/webhook` receives GitHub webhook events
- On `push` to a configured branch: trigger a spec run for any spec whose `code_under_test` paths overlap the changed files
- On `pull_request.opened` or `pull_request.synchronize`: trigger the relevant spec run and post result as a PR comment
- Webhook secret verified via HMAC (same pattern as EI-2)

**Config:** `[integrations.github]` in setup TOML. Personal access token or GitHub App credentials. Repo and branch filter.

### EI-6 — Run Scheduling

**Why:** Regression suites, memory benchmarks, and recurring spec sweeps should not require manual triggering.

- `scheduled_runs` table: `(schedule_id, spec_id, cron_expression, enabled, last_run_at, next_run_at)`
- `POST /api/schedules`, `GET /api/schedules`, `PUT /api/schedules/:id`, `DELETE /api/schedules/:id`
- Cron evaluator runs on a background tokio task; fires `POST /api/runs` when the schedule triggers
- Pack Board schedule manager panel: add/edit/disable schedules, see last run outcome

### EI-7 — Cost Budget Enforcement

**Why:** A misconfigured spec or an infinite fix loop can consume unbounded tokens. There is currently no hard stop mechanism.

- `max_cost_usd: Option<f64>` on `RunRequest` and in spec YAML
- After each LLM call, `get_run_cost_summary` checks accumulated cost against the budget. If exceeded: abort the current phase gracefully, write a `budget_exceeded` blocker to the blackboard, send a `run.failed` event with reason `budget_exceeded`
- `cost_hard_cap_usd` global config in setup TOML as a safety ceiling above any per-run budget
- Pack Board run overview shows budget consumed vs limit as a progress bar

### EI-8 — Health and Operational Endpoints

**Why:** Basic operational hygiene for any hosted or multi-machine deployment.

- `GET /health` — returns `{ status: "ok", version, uptime_secs, db_ok, memory_index_ok }`. Already exists as a concept; needs a real implementation that actually probes the DB and memory index.
- `GET /api/status` — authenticated endpoint returning active runs, agent claim count, memory entry count, last benchmark run
- CORS configuration in setup TOML: `[server.cors]` with `allowed_origins` list, defaulting to `localhost` only
- Structured JSON logging option (for log aggregators): `[server.logging] format = "json"` in setup TOML

---

## Parallel Product Track — Enterprise Integration (Siemens / Copilot Studio)

**Context:** The `setups/machines/caleb-siemens-laptop.toml` machine profile already exists and routes Claude as the default provider. The MCP server pattern is established in `harkonnen.toml`. This track formalizes the bridge between Harkonnen and the Siemens enterprise environment: Copilot Studio as a second control-plane surface, the internal knowledge base as a Coobie memory source, Microsoft Teams as the notification channel (replaces Slack in enterprise deployments), and Entra ID for auth.

The architecture is: Harkonnen exposes itself **as an MCP server** that Copilot Studio consumes. Copilot Studio already supports custom MCP servers (announced 2024–2025). This means no bespoke connector API is needed — the same MCP protocol that Claude Desktop and VS Code already use works natively inside Copilot Studio. Claude is accessed through the Anthropic API, configured in the Siemens machine profile.

This is independent of the consumer EI track. EI-1 (auth) should land first because every enterprise surface needs it.

---

### ENT-1 — Harkonnen as an MCP Server

**Why first:** MCP is the integration primitive. Once Harkonnen exposes itself as an MCP server, Copilot Studio, Claude Desktop, VS Code, and any other MCP-capable client can consume factory operations without bespoke connectors. This is the foundation for ENT-2 onward.

**What to build:**

- `src/mcp_server.rs` — implements the MCP server protocol (JSON-RPC 2.0 over stdio or SSE transport). The MCP spec has a Rust SDK (`rmcp` crate); use that rather than writing the transport layer by hand.
- **Resources** (read-only, queryable by clients):
  - `harkonnen://runs` — list of recent runs with status
  - `harkonnen://runs/{run_id}` — full run detail including traces, decisions, optimization program, metric attacks
  - `harkonnen://memory/lessons` — promoted lessons from the consolidation workbench
  - `harkonnen://memory/causal` — recent causal patterns Coobie has identified
  - `harkonnen://specs` — available specs for commissioning
- **Tools** (callable actions):
  - `run_spec(spec_id, options)` — triggers a factory run, returns run_id
  - `get_run_status(run_id)` — current phase + latest event
  - `resolve_checkpoint(checkpoint_id, decision, note)` — approve or reject a checkpoint from any MCP client
  - `ask_coobie(question, context)` — routes to Coobie's `dispatch_message`, returns the response
  - `ingest_memory(content, source, tags)` — pushes a document or note into Coobie's memory ingest pipeline
  - `list_decisions(run_id)` — returns the decision audit log for a run
- **Prompts** (parameterized prompt templates for Copilot Studio):
  - `briefing_for_spec(spec_id)` — pre-built Coobie briefing prompt
  - `diagnose_run(run_id)` — causal diagnosis prompt for a completed run
- MCP server transport registered in setup TOML under `[mcp.self]` — enables it when the flag is set:

```toml
[mcp.self]
enabled = true
transport = "sse"          # "stdio" for Claude Desktop / VS Code; "sse" for Copilot Studio
port = 3001                # separate port from the main HTTP API
auth_required = true       # reuses EI-1 API key
```

- CLI command `harkonnen mcp serve` starts the MCP server as a standalone process alongside the main server

**Done when:** Claude Desktop or VS Code can list factory runs, trigger a run, and ask Coobie a question via MCP tool calls. Copilot Studio can discover the server and invoke the same tools.

---

### ENT-2 — Copilot Studio Connector

**Why:** Copilot Studio agents compose actions from connectors. The MCP server (ENT-1) gives Copilot Studio native MCP access if the tenant has MCP support enabled. For tenants that don't yet have MCP preview access, a Power Platform custom connector backed by the Harkonnen REST API gives the same capability via OpenAPI.

**What to build:**

- `factory/copilot-studio/harkonnen-connector.json` — OpenAPI 3.0 spec covering the key Harkonnen endpoints: `POST /api/runs`, `GET /api/runs/:id`, `GET /api/runs/:id/traces`, `GET /api/runs/:id/decisions`, `POST /api/chat/threads/:id/messages`, `POST /api/coordination/checkpoints/:id/reply`
- `factory/copilot-studio/connector-manifest.yaml` — Power Platform connector manifest wrapping the OpenAPI spec. Includes display names, descriptions, and action categories in the format Copilot Studio's action library expects.
- `factory/copilot-studio/topic-templates/` — three starter Copilot Studio topic YAML files:
  - `run-spec.yaml` — triggers a factory run from a natural-language request, polls status, posts outcome
  - `ask-coobie.yaml` — routes a question to Coobie and surfaces the answer with run context
  - `checkpoint-review.yaml` — presents pending checkpoints and handles approve/reject inline in the conversation
- Authentication: OAuth2 client credentials flow using Entra ID (see ENT-3). Connector authenticates as a service principal, not as the individual user.
- Documentation at `factory/copilot-studio/README.md`: step-by-step setup for importing the connector into a Power Platform environment and wiring it to a Copilot Studio agent.

**Done when:** A Copilot Studio agent in the Siemens tenant can trigger a Harkonnen run, ask Coobie a question, and approve a checkpoint — all from a Teams or web chat window, without touching the Pack Board.

---

### ENT-3 — Entra ID Authentication

**Why:** In a Siemens enterprise environment, API keys are not acceptable for team or shared-service deployments. Entra ID (Azure AD) is the identity plane for everything Microsoft-adjacent. Copilot Studio, Teams bots, and Power Automate flows all authenticate via Entra ID service principals.

**What to build:**

- OAuth2/OIDC JWT validation middleware in `src/api.rs` — alongside the existing API key path. Either an API key **or** a valid Entra ID JWT is accepted on protected routes.
- `[auth.entra]` section in setup TOML:

```toml
[auth.entra]
enabled = true
tenant_id = "ENTRA_TENANT_ID"          # from env var
client_id = "ENTRA_CLIENT_ID"          # the registered app ID
audience  = "api://harkonnen-factory"  # must match the app's Application ID URI
```

- JWT validation: fetch JWKS from `https://login.microsoftonline.com/{tenant_id}/discovery/v2.0/keys`, validate signature, `aud`, `iss`, and expiry. Use the `jsonwebtoken` crate (already common in Rust ecosystems).
- Role claims: map Entra ID app roles to Harkonnen roles — `Harkonnen.Operator` (full access), `Harkonnen.Viewer` (read-only), `Harkonnen.Agent` (for service principal Copilot Studio connector)
- `GET /api/auth/me` — returns the authenticated identity and resolved role for debugging
- The Siemens machine profile (`setups/machines/caleb-siemens-laptop.toml`) gains an `[auth.entra]` section. Local dev can still use `HARKONNEN_API_KEY` for bypass.

**Done when:** A Copilot Studio connector authenticating as an Entra ID service principal can call the Harkonnen API without an API key, and a viewer-role principal cannot trigger runs or approve checkpoints.

---

### ENT-4 — Enterprise Knowledge Base Ingest (SharePoint / Graph API)

**Why:** The value of Coobie's memory is proportional to what's in it. In a Siemens context, the authoritative knowledge lives in SharePoint document libraries, Teams wikis, and internal project sites — not in files you can paste into the terminal. A Graph API connector makes that knowledge available to Coobie without manual re-entry.

**What to build:**

- `src/integrations/graph.rs` — Microsoft Graph API client. Authenticates via Entra ID client credentials (same service principal as ENT-3). Uses the Graph `v1.0` and `beta` endpoints.
- CLI command `harkonnen memory ingest --source sharepoint --site <site-url> --library <library-name>` — walks a SharePoint document library, downloads files, pipes each through the existing `memory ingest` extraction pipeline (text, PDF, OCR via Phase 5b), and writes results to Coobie's memory store.
- `harkonnen memory ingest --source teams-wiki --team <team-id> --channel <channel-id>` — ingests a Teams wiki tab as a structured memory document.
- Incremental sync: store the Graph API `deltaLink` from each sync in a `graph_sync_state` table. Re-running the command fetches only changed/added documents since the last run.
- `harkonnen memory ingest --source graph-search --query "<search terms>"` — queries Microsoft Search via Graph and ingests top results. Useful for pulling in content you don't know the exact location of.
- `[integrations.graph]` section in setup TOML:

```toml
[integrations.graph]
enabled = true
tenant_id = "ENTRA_TENANT_ID"
client_id = "ENTRA_CLIENT_ID"
client_secret_env = "ENTRA_CLIENT_SECRET"
```

- Bidirectional write-back (deferred to v2): after consolidation promotes a lesson, optionally write a summary back to a designated SharePoint list as a structured knowledge item. Operators can review and approve write-back separately from promoting within Harkonnen.

**Done when:** Running `harkonnen memory ingest --source sharepoint` against a Siemens document library adds its contents to Coobie's retrievable memory, and subsequent runs against related specs can cite those documents in the briefing.

---

### ENT-5 — Microsoft Teams Integration

**Why:** Siemens uses Teams, not Slack. The Teams integration replaces EI-3 in enterprise deployments. The delivery format is Adaptive Cards (Teams' equivalent of Slack Block Kit) rather than plain webhooks.

**Outbound (Teams notifies operator):**

- Adaptive Card on `run.completed`: outcome badge, agent trace count, cost, decision count, Coobie's top advisory concern, Pack Board button. Card is posted to a configured channel via the Teams Incoming Webhook URL.
- Checkpoint notification as an Actionable Adaptive Card — the operator can click Approve or Reject directly in the Teams message. The card's action calls back to `POST /api/coordination/checkpoints/:id/reply` using the ENT-3 service principal.
- `run.failed` card with Coobie's causal diagnosis summary and a "Diagnose in Coobie" deep link into PackChat.
- `metric_attack.detected` card: exploit description, detection signal, suggested mitigation, link to `GET /api/runs/:id/metric-attacks`.

**Inbound (Teams bot commands):**

- Bot registered as a Teams app in the Siemens tenant via the Bot Framework. Commands mirror the Slack interface:
  - `@Harkonnen run <spec-id>` — triggers run, replies with run ID card
  - `@Harkonnen status <run-id>` — current phase + last event
  - `@Harkonnen ask <question>` — routes to Coobie, replies with the response as a card
  - `@Harkonnen checkpoints` — lists open checkpoints for the current operator
- Bot can be scoped to a specific Teams channel or allowed globally within the tenant.

**Config:** `[integrations.teams]` in setup TOML:

```toml
[integrations.teams]
enabled = true
incoming_webhook_url_env = "TEAMS_WEBHOOK_URL"
bot_app_id_env           = "TEAMS_BOT_APP_ID"
bot_app_password_env     = "TEAMS_BOT_PASSWORD"
checkpoint_callback_base = "https://harkonnen.internal/api"
```

**Done when:** A completed run posts an Adaptive Card to the configured Teams channel, a checkpoint can be approved from within Teams without opening the Pack Board, and `@Harkonnen ask` routes to Coobie.

---

### ENT-6 — Siemens Machine Profile Hardening

**Why:** The `caleb-siemens-laptop.toml` profile exists but was generated from the `home-linux` template. It needs enterprise-specific config blocks for auth, integrations, and MCP self-exposure so that `cargo run -- setup check` on the Siemens machine validates the full enterprise stack.

**What to build:**

- Add `[auth.entra]` block to `caleb-siemens-laptop.toml` (populated from env vars, not hardcoded)
- Add `[integrations.teams]` block
- Add `[integrations.graph]` block
- Add `[mcp.self]` block with `transport = "sse"` and the Copilot Studio port
- Add a `siemens-enterprise` setup template to `setups/` so future Siemens machines can init from it rather than `home-linux`
- `cargo run -- setup check` extended to validate: Entra ID env vars present, Teams webhook reachable, Graph API credentials valid, MCP self-server starts cleanly

**Done when:** Running `cargo run -- setup check` on the Siemens laptop reports green for all enterprise integration checks, and a second Siemens machine can be provisioned from the `siemens-enterprise` template.

---

## Benchmark Track (cross-phase)

Benchmarks should advance in lockstep with implementation phases. When a phase ships, at least one benchmark gate tied to it should ship too.

### Phase-aligned milestones summary

| Phase | Key benchmarks unlocked |
| --- | --- |
| v1 | Decision audit completeness, memory supersession accuracy, WrongAnswer classification rate |
| Phase 2 | SWE-bench Verified readiness, LiveCodeBench, Aider Polyglot |
| Phase 3 | twin fidelity, hidden scenario delta, spec adherence rate, DevBench, coordination-compression / downstream-validation time |
| Phase 4b | StreamingQA belief-update accuracy |
| Phase 5b | FRAMES re-run (Qdrant), LongMemEval / LoCoMo regression check |
| Phase 6 | GAIA Level 3, AgentBench |
| Phase 7 | E-CARE, causal attribution accuracy |
| Phase 8 | unjustified drift, quarantine resolution quality, schema revision stability, stress / hysteresis recovery quality |

### Always-on benchmarks

- `Local Regression Gate` — hard merge gate, runs on every substantial change
- `LongMemEval` paired mode (Coobie vs raw LLM) — run on every memory-relevant change
- `LoCoMo QA` paired mode — longer-horizon memory regression check

### Competitive positioning benchmarks

#### vs Mem0 / MindPalace / Zep

- `FRAMES` — multi-hop factual recall; Mem0 publishes here. Native adapter live. Requires Phase 5b Qdrant for best results.
- `StreamingQA` — belief-update accuracy; no competitor tracks this. Phase 4b.
- `HELMET` — retrieval precision/recall. Native adapter live.
- `LongMemEval` — long-term assistant memory. Native adapter live.
- `LoCoMo QA` — long-horizon dialogue memory. Native adapter live.

#### vs OpenCode / Aider / single-agent coding tools

- `LiveCodeBench` — recent competitive programming problems; contamination-resistant. Phase 2.
- `Aider Polyglot` — Aider's own multi-language leaderboard. Phase 2.
- `DevBench` — full software lifecycle; structural argument against single-phase tools. Phase 3.
- `SWE-bench Verified` / `SWE-bench Pro` — industry-standard code loop benchmarks. Phase 2.

#### vs general agent frameworks

- `GAIA Level 3` — multi-step delegation; single-agent tools fail here. Phase 6.
- `AgentBench` — eight environments; tests Labrador role separation. Phase 6.

#### Causal reasoning — unique claim, no competitor benchmarks this

- `CLADDER` — Pearl hierarchy accuracy. Native adapter live.
- `E-CARE` — causal explanation coherence. Phase 7.

#### Harkonnen-native — cannot be run by any competitor

- `Spec Adherence Rate` — completeness and precision vs spec. Phase 3.
- `Hidden Scenario Delta` — visible vs hidden pass rate gap. Phase 3.
- `Causal Attribution Accuracy` — seeded failure corpus, top-1 / top-3. Phase 7.

### Reporting standard

Every reportable benchmark claim should include:

- the raw-LLM baseline on the same provider when meaningful
- the Harkonnen setup name and routing
- the benchmark split or slice used
- the commit hash and benchmark artifact path
- latency and cost where available, not just accuracy

---

## What is already done (do not redo)

**Gap-closure phases A–D (shipped 2026-04-18):**

- **A1** — `LlmUsage` struct; token + latency capture on all three providers; `run_cost_events` table; `GET /api/runs/:id/cost`
- **A2** — `DecisionRecord` struct; `decision_log` table; `record_decision` + `list_run_decisions`; `GET /api/runs/:id/decisions`; wired at plan critique and consolidation promotion
- **A3** — `Assignment` + `ClaimRequest` extended with `resource_kind`, `ttl_secs`, `guardrails`, `expires_at`; `POST /api/coordination/check-lease` handler with TTL expiry and guardrail pattern matching
- **B** — `AgentTrace` struct; `agent_traces` table + index; `record_agent_trace` + `list_run_traces`; `extract_reasoning()` parses `<reasoning>` blocks; wired at Scout intake, Coobie briefing, Coobie critique, Mason plan, Mason edits, Sable; `GET /api/runs/:id/traces`
- **C** — `OptimizationProgram` struct; `scout_derive_optimization_program` (LLM-backed, stub fallback); written to `optimization_program.json`; Coobie critique flags when plan doesn't address objective metric; `GET /api/runs/:id/optimization-program`
- **D** — `MetricAttack` struct; `sable_generate_metric_attacks` (2–3 attacks per run, exploit + detection signals + mitigations); written to `metric_attacks.json`; `GET /api/runs/:id/metric-attacks`

---

**Phase 1 — Core Factory + PackChat + Coobie Memory + Benchmark Toolchain:**

- Spec loading, validation, run lifecycle, SQLite persistence
- Phase-level attribution recording
- LLM routing for Claude, Gemini, OpenAI, and OpenAI-compatible local endpoints
- Scout, Mason, Piper, Sable, Ash, Flint LLM calls
- Mason opt-in file writes with staged workspace isolation
- Piper real build execution with stdout/stderr streaming
- Mason fix loop (up to 3 iterations on build failure)
- Live event broadcast (`LiveEvent`) + SSE endpoint `/api/runs/:id/events/stream`
- Coobie causal reasoning Phase 1 (heuristic rules, episode scoring)
- Coobie causal streaks and cross-run pattern detection
- Coobie Phase 3 preflight guidance (spec-scoped cause history → required checks)
- Coobie Palace (`src/coobie_palace.rs`) — den-based compound recall, patrol, scents
- Semantic memory (fastembed or OpenAI-compatible embeddings + SQLite vector store, hybrid retrieval)
- Causal feedback loop (causal reports + Sable rationale written back to project memory)
- Keeper coordination API (claims, heartbeats, conflict detection, release)
- Pack Board React UI (PackChat panel, Attribution Board, Factory Floor, Memory Board)
- PackChat backend: `chat_threads`, `chat_messages`, multi-turn `dispatch_message`, `@mention` routing, Coobie default fallback
- PackChat API routes: `GET/POST /api/chat/threads`, `GET /api/chat/threads/:id`, `GET/POST /api/chat/threads/:id/messages`, `POST /api/agents/:id/chat`
- Checkpoint/reply/unblock routes as PackChat control-plane backend
- Evidence bootstrap, annotation bundle validation, evidence promotion
- `harkonnen memory init` with pre-embedding on fresh clone
- First-class benchmark toolchain (`benchmark list/run/report`, manifest-driven suites, CI workflow)
- Native LongMemEval adapter + paired raw-LLM vs Harkonnen comparison mode
- Native LoCoMo QA adapter + paired raw-LLM vs Harkonnen comparison mode
- Native FRAMES adapter + paired raw-LLM vs Harkonnen comparison mode
- Native StreamingQA adapter (query-time invalidation reasons; persistence layer completing in v1-B)
- LM Studio / OpenAI-compatible benchmark routing for chat and embedding backends

**Phase 4 — Episodic Layer Enrichment + Causal Graph + Benchmarks:**

- `state_before` / `state_after` on `EpisodeRecord` and episodes table (workspace state snapshots via FNV-64 hash walk)
- `causal_links` table: `(link_id, run_id, from_event_id, to_event_id, relation, confidence, hierarchy_level, key, created_at)`
- `PearlHierarchyLevel` enum (Associational / Interventional / Counterfactual) on causal links
- `populate_cross_phase_causal_links` — auto-emits phase_sequence and failure_triggered links across run episodes
- `get_run_causal_graph` — returns event graph with Pearl-labeled edges; surfaced via `GET /api/runs/:id/causal-events`
- Coobie multi-hop retrieval: `retrieve_context_multihop(query, embedding_store, depth)` — configurable chain depth (1–3)
- Native CLADDER adapter — Pearl hierarchy causal benchmark, paired Harkonnen vs raw-LLM mode
- Native HELMET adapter — retrieval precision/recall benchmark

**Phase 5 — Consolidation Workbench:**

- `consolidation_candidates` table: `(candidate_id, run_id, kind, status, content_json, edited_json, confidence, label, created_at, reviewed_at)`
- `generate_consolidation_candidates`, `list_consolidation_candidates`, `review_consolidation_candidate`, `edit_consolidation_candidate`, `promote_kept_candidates`
- INSERT OR IGNORE idempotency on candidate generation
- API routes: `GET /api/runs/:id/consolidation/candidates`, `POST .../candidates` (generate), `POST .../candidates/:id/keep`, `.../discard`, `.../edit`, `POST /api/runs/:id/consolidate` (promote)
- Pack Board Consolidation Workbench panel: candidate cards with keep/discard/edit controls, confidence bars, expandable JSON, filter bar, promote footer
- `RunDetailDrawer` updated with workbench tab

---

## Tracking

Each active implementation phase gets its own git branch: `phase/v1-guardrails`, `phase/2-bramble-tests`, `phase/3-ash-twins`, etc.
A phase is merged to `main` when its "Done when" condition is verifiably met.
This file is updated when a phase ships — move it from the numbered list above into the "already done" section.

Benchmark wiring should advance in lockstep with implementation:

- when a phase ships, add or tighten at least one benchmark gate tied to it
- when a public benchmark is still adapter-only, capture that explicitly here rather than implying it is fully integrated
- benchmark artifacts belong in `factory/artifacts/benchmarks/` and should be linked from release notes once they support a public claim
