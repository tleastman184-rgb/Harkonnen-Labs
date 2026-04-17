# Operator Model Activation Plan

## Intent

Add a first-class pre-commissioning workflow that interviews the operator about how their work actually runs, saves approved answers as structured Harkonnen data, and generates agent-ready operating artifacts that Scout, Coobie, and Keeper can use before a run is commissioned.

This is the upstream layer missing from the current factory. Harkonnen already has:

- PackChat conversational control in `src/chat.rs`
- checkpoint approvals and unblock flow in `src/api.rs` and `src/orchestrator.rs`
- repo-local project memory and commissioning notes in `.harkonnen/project-memory/`
- Coobie preflight guardrails and required checks
- operator-reviewed consolidation after runs

What it does not have is a durable, structured way to capture the operator's working model before Scout drafts a spec.

## Why This Fits Harkonnen

Harkonnen should borrow the workflow shape from OB1's `work-operating-model-activation` recipe, but implement it natively in the local-first factory stack.

Keep:

- five-layer interview structure
- checkpoint approval after each layer
- structured persistence plus artifact exports
- compatibility with `operating-model.json`, `USER.md`, `SOUL.md`, `HEARTBEAT.md`, and `schedule-recommendations.json`

Do not copy:

- OB1 code, server layout, or Supabase edge-function deployment model
- any code whose reuse would create license ambiguity under OB1's current FSL-1.1-MIT terms

## Product Shape

## Product Defaults

The product should be explicitly **project-first**.

Default behavior:

- start or resume a `project` profile tied to the commissioned repo
- export artifacts into that repo's `.harkonnen/operator-model/` directory
- feed the stamped project profile into Scout, Coobie, and Keeper for that repo

Global behavior should stay intentionally light and optional:

- only create a `global` profile when the operator asks for a reusable baseline
- keep it focused on broad preferences: communication style, approval boundaries, escalation style, and working rhythm
- never let the global profile replace repo-specific workflows, dependencies, institutional knowledge, or friction

Resolution order for commissioning and preflight:

1. matching `project` profile for the target repo
2. otherwise a light `global` profile if one exists
3. otherwise no operator model yet

### Entry points

1. `New Run` flow gets a new first choice:
   - `Draft Spec Now`
   - `Interview Me First`
2. PackChat gets a new thread kind: `operator_model`
3. Coobie can be directly addressed with a bootstrapping prompt like:
   - `@coobie interview me to build my operating model`

### Interview layers

Run these in fixed order with approval gates:

1. operating rhythms
2. recurring decisions
3. dependencies
4. institutional knowledge
5. friction

Each layer produces:

- free-text checkpoint summary for operator approval
- canonical structured entries
- one summary memory write to repo-local project memory by default, or core memory only for the rare light global profile

### Outputs

At successful completion, generate and persist:

- `operating-model.json`
- `USER.md`
- `SOUL.md`
- `HEARTBEAT.md`
- `schedule-recommendations.json`
- `commissioning-brief.json` (Harkonnen-specific)

For `project` scope these should land in the target repo under `.harkonnen/operator-model/`.
For the optional light `global` scope they can live under Harkonnen's own factory state.

`commissioning-brief.json` should be the bridge artifact Scout and Coobie use before spec drafting.

## Proposed Data Model

Add a native operator-model store in SQLite.

### Tables

#### `operator_model_profiles`

One logical profile per human or per repo-scoped operator context. In practice, `project` should be the default and `global` should stay small.

Fields:

- `profile_id TEXT PRIMARY KEY`
- `scope TEXT NOT NULL`  
  `global` or `project`
- `project_root TEXT`  
  null for global profiles
- `display_name TEXT NOT NULL DEFAULT ''`
- `status TEXT NOT NULL DEFAULT 'active'`
- `current_version INTEGER NOT NULL DEFAULT 0`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

#### `operator_model_sessions`

A resumable interview run.

Fields:

- `session_id TEXT PRIMARY KEY`
- `profile_id TEXT NOT NULL`
- `thread_id TEXT`  
  linked PackChat thread
- `status TEXT NOT NULL`  
  `active`, `review`, `completed`, `abandoned`
- `pending_layer TEXT`
- `started_by TEXT`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `completed_at TEXT`

#### `operator_model_layer_checkpoints`

One approved checkpoint per layer per version.

Fields:

- `checkpoint_id TEXT PRIMARY KEY`
- `session_id TEXT NOT NULL`
- `profile_id TEXT NOT NULL`
- `version INTEGER NOT NULL`
- `layer TEXT NOT NULL`
- `status TEXT NOT NULL`  
  `draft`, `approved`, `superseded`
- `summary_md TEXT NOT NULL`
- `raw_notes_json TEXT NOT NULL`
- `approved_by TEXT`
- `created_at TEXT NOT NULL`
- `approved_at TEXT`

#### `operator_model_entries`

Canonical structured facts extracted from a checkpoint.

Fields:

- `entry_id TEXT PRIMARY KEY`
- `profile_id TEXT NOT NULL`
- `version INTEGER NOT NULL`
- `layer TEXT NOT NULL`
- `entry_type TEXT NOT NULL`
- `title TEXT NOT NULL`
- `content TEXT NOT NULL`
- `details_json TEXT NOT NULL`
- `source_checkpoint_id TEXT NOT NULL`
- `status TEXT NOT NULL DEFAULT 'current'`
- `superseded_by TEXT`
- `created_at TEXT NOT NULL`

Suggested `entry_type` values:

- `cadence`
- `decision_rule`
- `dependency`
- `institutional_fact`
- `friction`
- `boundary`
- `escalation_rule`

#### `operator_model_exports`

Persist exported artifacts by version.

Fields:

- `export_id TEXT PRIMARY KEY`
- `profile_id TEXT NOT NULL`
- `version INTEGER NOT NULL`
- `artifact_name TEXT NOT NULL`
- `content TEXT NOT NULL`
- `content_type TEXT NOT NULL`
- `created_at TEXT NOT NULL`

#### `operator_model_update_candidates`

Review queue for suggested updates inferred from runs and consolidation.

Fields:

- `candidate_id TEXT PRIMARY KEY`
- `profile_id TEXT NOT NULL`
- `run_id TEXT`
- `entry_id TEXT`
- `proposal_kind TEXT NOT NULL`  
  `add`, `edit`, `supersede`, `archive`
- `summary TEXT NOT NULL`
- `proposal_json TEXT NOT NULL`
- `status TEXT NOT NULL DEFAULT 'open'`
- `confidence REAL NOT NULL DEFAULT 0`
- `created_at TEXT NOT NULL`
- `reviewed_at TEXT`

## PackChat Integration

Reuse the existing chat and checkpoint model instead of building a parallel interview system.

### Minimal chat changes

#### `src/db.rs`

Add to `chat_threads`:

- `thread_kind TEXT NOT NULL DEFAULT 'general'`
- `metadata_json TEXT`

Thread kinds:

- `general`
- `run`
- `spec`
- `operator_model`

#### `src/chat.rs`

Extend:

- `ChatThread`
- `OpenThreadRequest`

to carry `thread_kind` and optional metadata.

### Interview behavior

The interview should run inside a standard PackChat thread with the following mechanics:

1. operator opens or resumes an `operator_model` thread
2. Coobie owns the interview by default
3. after each layer Coobie posts a checkpoint summary
4. operator approval is persisted through the existing checkpoint reply pattern
5. approved layer is written atomically to the operator-model tables

## API Surface

Add a dedicated operator-model API namespace.

### Session and profile routes

- `POST /api/operator-model/sessions`
  starts or resumes the active interview
- `GET /api/operator-model/profiles`
  lists profiles
- `GET /api/operator-model/profiles/:id`
  returns profile summary, latest version, completion status, last exports
- `GET /api/operator-model/profiles/:id/query`
  queries slices by layer, entry type, keyword, cadence, stakeholder, unresolved friction

### Layer and approval routes

- `POST /api/operator-model/sessions/:id/layers/:layer/draft`
  saves draft layer synthesis
- `POST /api/operator-model/sessions/:id/layers/:layer/approve`
  approves and canonicalizes layer
- `POST /api/operator-model/sessions/:id/complete`
  triggers contradiction pass and export generation

### Export and update routes

- `GET /api/operator-model/profiles/:id/exports`
- `GET /api/operator-model/profiles/:id/exports/:artifact`
- `GET /api/operator-model/profiles/:id/update-candidates`
- `POST /api/operator-model/update-candidates/:id/keep`
- `POST /api/operator-model/update-candidates/:id/discard`
- `POST /api/operator-model/update-candidates/:id/edit`

### Import/export compatibility routes

- `POST /api/operator-model/import`
  accepts OB1-style artifacts as input
- `GET /api/operator-model/profiles/:id/export-bundle`
  returns the Harkonnen/OB1-compatible artifact pack

## Orchestrator and Coobie Integration

### Before spec drafting

When `Interview Me First` is used:

1. start or resume the `project` operator-model session for the target repo
2. finish or resume interview
3. generate `commissioning-brief.json` in that repo's `.harkonnen/operator-model/` directory
4. pass the brief into Scout draft generation

Scout should treat the operator model as a hard context source, not just loose memory.

### Before run preflight

Coobie should read the latest approved `project` operator model alongside repo-local project memory and causal priors, falling back to the light global baseline only when no project profile exists yet.

Use it to shape:

- `required_checks`
- `guardrails`
- escalation rules
- definitions of done
- recurring dependency timing assumptions

### During runs

Keeper should use operator-model boundary and escalation entries when deciding whether to block or request clarification.

### After runs

Coobie consolidation should emit `operator_model_update_candidates` when a run reveals:

- new recurring blockers
- stale dependencies
- superseded decision rules
- repeated friction patterns

## Memory Integration

### Where the knowledge lives

Use both structured tables and memory writes.

- structured tables are the canonical source for querying and export generation
- project memory stores summary lessons for the default repo-scoped profile; core memory is only for the optional light global baseline

### Write strategy

Per approved layer:

- write one structured checkpoint
- write canonical entries
- write one summary memory note tagged with:
  - `operator-model`
  - layer name
  - scope
  - profile/version

### Update strategy

If a later version changes a canonical entry:

- mark the older entry `superseded`
- link `superseded_by`
- write one `memory_updates` style summary note

This should explicitly reuse the stale-memory/supersession behavior already present in `src/memory.rs`.

## UI Changes

### `ui/src/components/NewRunFlow.jsx`

Add step `0a` before `Describe`:

- toggle or card choice: `Draft Spec Now` vs `Interview Me First`
- if interview chosen, launch the repo-scoped operator-model session and route into a conversational modal

### New component: `ui/src/components/OperatorModelFlow.jsx`

Responsibilities:

- display current layer
- render live PackChat interview transcript
- show approval checkpoint cards
- allow resume after interruption
- show progress across five layers

### `ui/src/components/RunDetailDrawer.jsx`

Add an `operator-model` tab when a run references a profile.

Show:

- profile version used for this run
- linked `commissioning-brief.json`
- operator-model assumptions consumed by Scout and Coobie
- update candidates generated from this run

### Pack Board additions

Add an `Operator Model` panel or drawer entry showing:

- active profile
- latest version
- incomplete interview sessions
- open update candidates
- exported artifacts

## Proposed Native Files

### New backend files

- `src/operator_model.rs`  
  service layer for sessions, persistence, exports, contradiction pass, and update proposals
- `src/operator_model_exports.rs`  
  artifact rendering helpers if `src/operator_model.rs` grows too large

### Existing backend files to modify

- `src/main.rs`  
  register new module
- `src/models.rs`  
  add operator-model DTOs and response models
- `src/db.rs`  
  create tables and migrations
- `src/chat.rs`  
  support `thread_kind` and metadata
- `src/api.rs`  
  add operator-model routes
- `src/orchestrator.rs`  
  read operator model during Scout draft and Coobie preflight; emit update candidates after consolidation

### New frontend files

- `ui/src/components/OperatorModelFlow.jsx`
- `ui/src/components/OperatorModelPanel.jsx`
- optionally `ui/src/components/OperatorModelArtifacts.jsx`

### Existing frontend files to modify

- `ui/src/components/NewRunFlow.jsx`
- `ui/src/components/RunDetailDrawer.jsx`
- `ui/src/App.jsx`  
  if a global drawer or panel entry is added

## Suggested Build Order

### Slice 1 — Storage and models

Files:

- `src/models.rs`
- `src/db.rs`
- `src/operator_model.rs` (initial skeleton)
- `src/main.rs`

Deliverable:

- migrations compile
- CRUD works from unit-level service methods
- export rendering works from fixture data

### Slice 2 — API and PackChat plumbing

Files:

- `src/chat.rs`
- `src/api.rs`
- `src/operator_model.rs`

Deliverable:

- operator-model sessions can start, resume, approve layers, and export artifacts through HTTP
- chat threads can be typed as `operator_model`

### Slice 3 — UI interview flow

Files:

- `ui/src/components/OperatorModelFlow.jsx`
- `ui/src/components/NewRunFlow.jsx`
- `ui/src/App.jsx` if needed

Deliverable:

- user can complete a five-layer interview from the existing `New Run` path
- progress and approval state survive refresh/reopen

### Slice 4 — Scout and Coobie integration

Files:

- `src/orchestrator.rs`
- `src/api.rs`

Deliverable:

- Scout draft can consume `commissioning-brief.json`
- Coobie preflight surfaces operator-model-derived checks distinctly from causal priors

### Slice 5 — Review and update loop

Files:

- `src/orchestrator.rs`
- `src/operator_model.rs`
- `ui/src/components/RunDetailDrawer.jsx`
- `ui/src/components/OperatorModelPanel.jsx`

Deliverable:

- runs can propose operator-model updates
- operator can keep/discard/edit those proposals
- accepted proposals create a new profile version or supersede entries in place

### Slice 6 — OB1 interoperability

Files:

- `src/api.rs`
- `src/operator_model.rs`
- tests/fixtures for import/export payloads

Deliverable:

- import OB1-style artifact bundle
- export Harkonnen bundle matching the same artifact names

## Acceptance Criteria

1. A user can choose `Interview Me First` from the current commissioning path.
2. The interview runs in five fixed layers with approval after each layer.
3. Approved layers persist in SQLite and write summary notes into Harkonnen memory.
4. The system emits `operating-model.json`, `USER.md`, `SOUL.md`, `HEARTBEAT.md`, `schedule-recommendations.json`, and `commissioning-brief.json`.
5. Scout uses the latest approved operator model when drafting specs.
6. Coobie preflight uses the latest approved operator model when generating guardrails and `required_checks`.
7. Post-run consolidation can propose operator-model updates.
8. Superseded operator assumptions are visible as stale/superseded, not silently replaced.

## Risks and Mitigations

### Risk: interview becomes a second product instead of a factory primitive

Mitigation:

- keep it inside PackChat and `New Run`
- reuse existing checkpoint and review mechanics

### Risk: operator model duplicates project memory

Mitigation:

- structured operator model is for reusable work patterns and decision logic
- project memory remains the home for repo/runtime/domain facts

### Risk: too much tool surface for Coobie

Mitigation:

- query operator model through one focused service/router instead of exposing many small tools
- keep PackChat routing simple: Coobie owns interview, Scout consumes outputs

### Risk: license confusion with OB1

Mitigation:

- implement natively
- limit reuse to workflow ideas and artifact compatibility
- review legal posture before any direct code borrowing

## Recommended Roadmap Placement

Treat this as a parallel product/control-plane track, not a benchmark phase replacement.

Recommended slot:

- start after the current active Phase 2 and Phase 3 work begins stabilizing
- ship Slice 1-3 as a product milestone
- ship Slice 4-6 as a follow-on milestone

This keeps the benchmark and twin/test roadmap intact while making Harkonnen dramatically easier to commission.
