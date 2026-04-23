# Phase 5-C — BriefingScope Design

## Purpose

Every agent currently receives the same Coobie preflight briefing regardless of
role or phase. This is wasteful (irrelevant context burns context window) and
unsafe (Sable must never see Mason's implementation reasoning before scoring
hidden scenarios). Phase 5-C introduces a `BriefingScope` enum that shapes
retrieval at the call site, not in storage.

This document specifies the enum variants, allow/deny matrices, wire-up
points in the orchestrator, and the `briefing_scope` field added to episode
records for downstream causal analysis.

---

## Enum Definition

```rust
// src/coobie.rs (migrates to src/memory/briefing.rs in Phase 5b)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BriefingScope {
    ScoutPreflight,
    MasonPreflight,
    PiperPreflight,
    BramblePreflight,
    SablePreflight,
    AshPreflight,
    FlintPreflight,
    CoobiePreflight,       // Coobie self-briefing before episode write
    CooobieConsolidation,  // Consolidation candidate review
    OperatorQuery,         // Ad-hoc PackChat / operator question
}
```

Each variant carries an implicit `phase_id` (which run phase it belongs to)
and a `role` tag for logging and causal attribution.

---

## Allow/Deny Matrix

Each scope defines:
- `allow_categories` — memory hit tags that are included
- `deny_categories` — tags that are explicitly excluded regardless of relevance score

A retrieved hit whose tag set intersects `deny_categories` is **dropped before
the briefing is assembled**, not ranked lower. For Sable this is non-negotiable.

### ScoutPreflight

```yaml
scope: ScoutPreflight
allow_categories:
  - spec_history
  - prior_ambiguities
  - operator_model
  - open_questions
  - causal_links
  - project_posture        # from .harkonnen/repo.toml stamp
  - failure_patterns       # spec-level, not implementation-level
deny_categories:
  - implementation_notes
  - mason_plan
  - edit_rationale
  - test_results
  - scenario_outcome
inject_sources:
  - repo_stamp_mythos      # purpose, vertical, stakes from repo.toml
  - repo_stamp_ethos       # prohibitions, approved MCP posture
  - commissioning_brief    # top-3 operator patterns if present
max_hits: 12
```

### MasonPreflight

```yaml
scope: MasonPreflight
allow_categories:
  - failure_patterns
  - fix_patterns
  - workspace_guardrails
  - causal_links
  - wrong_answer_diffs
  - implementation_notes   # Mason CAN see prior implementation lessons
  - mason_plan             # Mason CAN see prior plan patterns
  - edit_rationale         # Mason CAN see prior edit rationale
  - operator_model         # risk tolerances, preferred tools
deny_categories:
  - scenario_outcome       # Mason must not see Sable's pass/fail
  - hidden_scenario_patterns
inject_sources:
  - repo_stamp_praxis      # prior behavioral expression patterns
  - commissioning_brief
max_hits: 15
```

### PiperPreflight

```yaml
scope: PiperPreflight
allow_categories:
  - tool_failure_patterns
  - environment_issues
  - build_tool_patterns
  - causal_links
deny_categories:
  - implementation_notes
  - mason_plan
  - edit_rationale
  - scenario_outcome
  - hidden_scenario_patterns
inject_sources: []
max_hits: 8
```

### BramblePreflight

```yaml
scope: BramblePreflight
allow_categories:
  - test_failure_patterns
  - coverage_patterns
  - wrong_answer_diffs
  - causal_links
deny_categories:
  - implementation_notes
  - mason_plan
  - edit_rationale
  - scenario_outcome
  - hidden_scenario_patterns
inject_sources: []
max_hits: 8
```

### SablePreflight  ← ISOLATION CONSTRAINT (non-negotiable)

```yaml
scope: SablePreflight
allow_categories:
  - scenario_patterns
  - hidden_scenario_outcomes
  - causal_links           # spec-level causality only, not implementation causality
  - metric_attack_patterns
deny_categories:
  - implementation_notes   # HARD BLOCK — must never reach Sable
  - mason_plan             # HARD BLOCK
  - edit_rationale         # HARD BLOCK
  - fix_patterns           # HARD BLOCK — contains implementation reasoning
  - wrong_answer_diffs     # HARD BLOCK — reveals what Mason tried
  - operator_model         # exclude to prevent steering Sable toward operator preference
inject_sources: []         # no stamp injection — Sable's context must be scenario-pure
max_hits: 10
isolation_enforcement: strict
  # If any retrieved hit's tag set intersects the deny_categories list,
  # the entire hit is dropped and a warning is emitted to the run log.
  # This is NOT a soft rank penalty — it is a hard exclusion.
```

### AshPreflight

```yaml
scope: AshPreflight
allow_categories:
  - twin_failure_patterns
  - service_stub_patterns
  - environment_issues
  - causal_links
deny_categories:
  - implementation_notes
  - mason_plan
  - edit_rationale
  - scenario_outcome
  - hidden_scenario_patterns
inject_sources: []
max_hits: 8
```

### FlintPreflight

```yaml
scope: FlintPreflight
allow_categories:
  - documentation_patterns
  - artifact_patterns
  - spec_history
deny_categories:
  - implementation_notes
  - mason_plan
  - edit_rationale
  - scenario_outcome
  - hidden_scenario_patterns
inject_sources: []
max_hits: 6
```

### CooobiePreflight / CooobieConsolidation

```yaml
scope: CooobiePreflight
allow_categories:
  - all                    # Coobie sees all categories; she wrote them
deny_categories: []
inject_sources:
  - repo_stamp_full
  - commissioning_brief
max_hits: 20

scope: CooobieConsolidation
allow_categories:
  - all
deny_categories: []
inject_sources:
  - repo_stamp_full
max_hits: 25
```

### OperatorQuery

```yaml
scope: OperatorQuery
allow_categories:
  - all
deny_categories: []
inject_sources:
  - commissioning_brief
max_hits: 20
```

---

## API

```rust
// Replaces the current single-path build_preflight_briefing()

pub async fn build_scoped_briefing(
    scope: BriefingScope,
    run_id: &str,
    spec_id: &str,
    context: &AppContext,
) -> Result<BriefingPackage>
```

`BriefingPackage` contains:
```rust
pub struct BriefingPackage {
    pub scope: BriefingScope,
    pub run_id: String,
    pub spec_id: String,
    pub hits: Vec<MemoryRetrievalHit>,          // filtered by allow/deny
    pub injected_sources: Vec<InjectedContext>, // repo stamp, commissioning brief
    pub hit_count_before_filter: usize,
    pub hit_count_after_filter: usize,
    pub denied_hit_count: usize,               // for audit log
    pub isolation_warnings: Vec<String>,        // if SablePreflight denied a hit
    pub assembled_text: String,                 // final briefing text for LLM prompt
}
```

Internally, `build_scoped_briefing`:
1. Runs the existing multi-hop retrieval chain against `spec_id` query
2. Filters hits against the scope's `deny_categories` (hard drop, log warnings)
3. Filters hits against `allow_categories` (include only matching)
4. Loads `inject_sources` (repo stamp from `.harkonnen/repo.toml`, commissioning brief)
5. Assembles `assembled_text` from filtered hits + injected sources
6. Returns `BriefingPackage` with audit fields populated

---

## Orchestrator Wire-Up Points

```rust
// Scout phase
let briefing = build_scoped_briefing(
    BriefingScope::ScoutPreflight, &run_id, &spec_id, &ctx
).await?;

// Mason phase
let briefing = build_scoped_briefing(
    BriefingScope::MasonPreflight, &run_id, &spec_id, &ctx
).await?;

// Sable phase
let briefing = build_scoped_briefing(
    BriefingScope::SablePreflight, &run_id, &spec_id, &ctx
).await?;
// Assertion: briefing.isolation_warnings.is_empty() — emit alarm if not
```

Piper, Bramble, Ash, Flint default to their respective scopes. Agents that had
no briefing previously (Piper, Flint) can receive a thin-scope briefing for the
first time in this phase — max_hits 6–8 keeps context pressure low.

---

## Repo Stamp Injection (`.harkonnen/repo.toml`)

The stamped project interview context belongs in `.harkonnen/repo.toml`.
Fields that flow into Scout and Coobie briefings:

```toml
[project]
purpose        = "..."
vertical       = "..."
primary_stakes = ["..."]

[ethos]
prohibitions   = ["..."]
approved_mcp   = ["..."]

[pathos]
stakeholder_attitudes = ["..."]
operator_risk_profile = "conservative|moderate|aggressive"

[praxis]
preferred_tools = ["..."]
skill_sources   = ["..."]
```

`inject_sources` mapping:
- `repo_stamp_mythos` → `[project]` section → purpose, vertical, stakes
- `repo_stamp_ethos` → `[ethos]` section → prohibitions, approved MCP
- `repo_stamp_praxis` → `[praxis]` section → preferred tools, skill sources
- `repo_stamp_full` → all sections

---

## Episode Record: `briefing_scope` Field

Add to `EpisodeRecord` and `phase_attributions` table:

```rust
pub struct EpisodeRecord {
    // ... existing fields ...
    pub briefing_scope: Option<String>,  // serialized BriefingScope variant name
}
```

SQLite migration:
```sql
ALTER TABLE phase_attributions ADD COLUMN briefing_scope TEXT;
```

Purpose: causal analysis can distinguish whether a lesson was visible at the
relevant phase. Example query: "Did Mason's fix pattern appear in Mason's
briefing on the run where it was applied?" This is the signal that tells Coobie
whether the briefing actually helped.

---

## Done When

- Scout, Mason, and Sable each receive a distinct briefing shaped to their role
- `briefing.isolation_warnings` is empty on every Sable briefing (verified in
  the run log)
- A log entry records `briefing_scope`, `hit_count_before_filter`,
  `hit_count_after_filter`, and `denied_hit_count` per phase
- Stamped repo interview context is visible in Scout and Coobie briefing text
- `briefing_scope` field present on `phase_attributions` rows
- Existing tests continue to pass (no behavior change for non-Sable phases
  until Phase 5b memory refactor)
