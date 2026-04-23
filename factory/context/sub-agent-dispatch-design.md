# Sub-Agent Dispatch Design

## Problem

The orchestrator's main context window inflates as each Labrador phase runs.
The most expensive offenders:

1. **Coobie briefing construction** — loads memory index + N `.md` files to
   assemble a briefing; all that raw content lives in the orchestrator context
   even though only the finished briefing text is needed downstream
2. **Sable scenario evaluation** — reads hidden scenario files, run artifacts,
   and causal history; the isolation requirement makes context leakage a
   correctness problem, not just a size problem
3. **Mason failure diagnosis** — on a complex WrongAnswer failure, Mason may
   trace through many files to understand the diff; that exploration should
   not linger in the orchestrator context across subsequent phases

## Solution

A `SubAgentDispatcher` abstraction that wraps each high-context task in an
isolated sub-agent invocation. The orchestrator calls `dispatcher.dispatch(task,
input)` and receives only the finished output — the exploration stays in the
sub-agent's context window and is discarded when the sub-agent completes.

The dispatcher is **pluggable per task**: different backends can be configured
for different tasks and different environments without changing orchestrator
code. The existing `[routing.agents]` TOML pattern is extended with a
`[sub_agents]` section.

---

## Architecture

### Sub-agent task types

```rust
// src/subagent.rs

pub enum SubAgentTask {
    BriefingConstruction,   // Coobie: load memory, filter by scope, return text
    EpisodeCapture,         // Coobie: write episode after run (low-context; usually direct)
    ScenarioEvaluation,     // Sable: evaluate run against hidden scenarios
    FailureDiagnosis,       // Mason: diagnose WrongAnswer/compile failure from diff
    SpecIntake,             // Scout: parse spec, surface ambiguities
    PolicyCheck,            // Keeper: check action against policy
    Custom(String),         // escape hatch for one-off tasks
}
```

### Backend enum

```rust
pub enum SubAgentBackend {
    /// Current behavior — LLM call in orchestrator context, no isolation.
    DirectLlm,

    /// Claude Code Agent tool spawn. Isolated context window.
    /// Input: system_prompt + task description.
    /// Output: single summary message returned to orchestrator.
    ClaudeCodeAgent {
        model: String,         // e.g. "claude-sonnet-4-6"
        max_turns: Option<u32>,
    },

    /// Codex CLI in plan mode. Best for implementation planning and diagnosis.
    /// Isolated process; output captured from stdout.
    CodexPlanAgent {
        model: String,         // e.g. "codex-mini-latest"
        context_paths: Vec<String>,  // files to include in Codex context window
    },

    /// Gemini via Agent SDK or direct API call.
    GeminiAgent {
        model: String,         // e.g. "gemini-2.5-pro"
    },

    /// Route via an MCP tool on a named server.
    /// Useful for hosted agents or external orchestrators.
    ExternalMcp {
        server: String,
        tool: String,
    },
}
```

### Input/output contract

```rust
pub struct SubAgentInput {
    pub task: SubAgentTask,
    pub system_prompt: String,    // built by subagent.rs prompt builders
    pub task_description: String, // the specific work for this invocation
    pub context: SubAgentContext, // structured input data
}

pub struct SubAgentContext {
    pub run_id: String,
    pub spec_id: String,
    pub phase: String,
    pub keywords: Vec<String>,    // for briefing retrieval scoping
    pub artifact_paths: Vec<String>, // for evaluation/diagnosis tasks
    pub extra: serde_json::Value, // task-specific structured data
}

pub struct SubAgentResult {
    pub task: SubAgentTask,
    pub backend_used: SubAgentBackend,
    pub output: String,           // the summary text returned to orchestrator
    pub structured: Option<serde_json::Value>, // parsed structured output if any
    pub tokens_used: Option<u64>, // if available from backend
    pub duration_ms: u64,
}
```

### Memory write discipline (non-negotiable)

Sub-agents **read** from memory. Only the orchestrator **writes**.

```rust
// Enforced two ways:
// 1. Sub-agent system prompts include disallowed_tools: [memory_write, db_write]
// 2. The memory and SQLite write methods are not exposed to sub-agent tool configs

// Pattern: orchestrator receives SubAgentResult, then decides what to persist
let result = dispatcher.dispatch(SubAgentTask::BriefingConstruction, input).await?;
// result.output is the finished briefing text
// Coobie writes the episode AFTER the run, in the orchestrator's own call
```

This maps to supervised-autonomy: sub-agents explore and report; the operator
(orchestrator) commits.

---

## Configuration

### `harkonnen.toml` extension

```toml
[sub_agents]
# Global default — use DirectLlm unless overridden
default_mode = "direct_llm"

# Coobie briefing construction — first Phase 5-C use case
[sub_agents.coobie_briefing]
task    = "briefing_construction"
backend = "claude_code_agent"
model   = "claude-sonnet-4-6"
isolation = true
memory_write = false

# Sable evaluation — isolation-critical
[sub_agents.sable_evaluation]
task    = "scenario_evaluation"
backend = "claude_code_agent"
model   = "claude-opus-4-7"
isolation = true
memory_write = false

# Mason diagnosis — Codex is often better at tracing implementation failures
[sub_agents.mason_diagnosis]
task    = "failure_diagnosis"
backend = "codex_plan_agent"
model   = "codex-mini-latest"
context_paths = ["src/", "tests/"]
isolation = true
memory_write = false

# Scout intake — can use Claude or Codex depending on env
[sub_agents.scout_intake]
task    = "spec_intake"
backend = "direct_llm"   # low-context; isolation optional
memory_write = false
```

These can be overridden per environment (home-linux vs work-windows) using the
same named-setup pattern already in `setups/`:

```toml
# setups/work-windows.toml overrides:
[sub_agents.mason_diagnosis]
backend = "codex_plan_agent"
model   = "o4-mini"
```

### Agent profile extension (optional)

Agent profiles can declare per-task dispatch preferences that take priority
over the global `[sub_agents]` config:

```yaml
# factory/agents/profiles/coobie.yaml
name: coobie
provider: claude-sonnet

dispatch:
  briefing_construction:
    backend: claude_code_agent
    isolation: true
  episode_capture:
    backend: direct_llm    # low-context; no isolation needed
```

The resolution order:
1. Agent profile `dispatch.<task>` (most specific)
2. `[sub_agents.<task_name>]` in harkonnen.toml
3. `[sub_agents] default_mode`

---

## Orchestrator integration

The existing phase entry points in `orchestrator.rs` replace direct briefing
calls with dispatcher calls:

```rust
// Phase 5-C: Scout phase
let dispatcher = ctx.sub_agent_dispatcher();
let briefing = dispatcher.dispatch(
    SubAgentTask::BriefingConstruction,
    SubAgentInput {
        task: SubAgentTask::BriefingConstruction,
        system_prompt: subagent::coobie_briefing_prompt(&run_id, "scout", &keywords),
        task_description: format!("Build ScoutPreflight briefing for spec {spec_id}"),
        context: SubAgentContext { run_id, spec_id, phase: "scout".into(), keywords, .. },
    }
).await?;

// Phase 5-C: Sable phase
let eval_result = dispatcher.dispatch(
    SubAgentTask::ScenarioEvaluation,
    SubAgentInput {
        system_prompt: subagent::sable_prompt(&run_id, &artifact_path),
        task_description: format!("Evaluate run {run_id} against hidden scenarios"),
        context: SubAgentContext { artifact_paths: vec![artifact_path], .. },
        ..
    }
).await?;

// Phase 2: Mason WrongAnswer diagnosis
if failure_kind == FailureKind::WrongAnswer {
    let diagnosis = dispatcher.dispatch(
        SubAgentTask::FailureDiagnosis,
        SubAgentInput {
            system_prompt: subagent::mason_diagnosis_prompt(&run_id, &diff),
            task_description: "Diagnose WrongAnswer failure and propose targeted fix".into(),
            ..
        }
    ).await?;
    // pass diagnosis.output as context to Mason's diff-focused fix prompt
}
```

---

## Backend implementations

### `ClaudeCodeAgent` backend

Uses the Claude Code Agent tool pattern (spawns isolated conversation, returns
summary). In Rust, this means:

1. Call the Anthropic API with the sub-agent's `system_prompt` + `task_description`
2. Allow the sub-agent to use its configured tool set (filesystem read, MCP)
3. Collect the final message as `SubAgentResult.output`
4. Discard the intermediate turns

For Claude Code sessions (this tool), the `Agent` tool in the CLAUDE.md skills
handles this directly. For the Rust orchestrator running Mason/Piper/etc. as
headless calls, this becomes an Anthropic API call with the agent's system
prompt as the `system` parameter and the task description as the first user
message.

### `CodexPlanAgent` backend

Invokes the Codex CLI with `--plan` flag and the task description. Returns
stdout as `SubAgentResult.output`. The `context_paths` list is passed as
`--context` arguments so Codex loads only the relevant files, not the whole
repo.

```sh
codex --plan --model o4-mini \
  --context src/ --context tests/ \
  "Diagnose WrongAnswer failure: expected FizzBuzz at index 14, got Fizz. Diff: ..."
```

### `DirectLlm` backend (fallback / current behavior)

Calls the configured LLM provider directly via `llm.rs`, no isolation. Used
for low-context tasks where isolation overhead isn't justified (Keeper policy
checks, Flint packaging instructions, simple Piper tool calls).

---

## Phase rollout

### Phase 5-C (first use)

Wire dispatcher for two tasks:
- `BriefingConstruction` → `ClaudeCodeAgent` (biggest context win; BriefingScope
  work already designed)
- `ScenarioEvaluation` → `ClaudeCodeAgent` (isolation-critical for Sable)

Config defaults go into `harkonnen.toml`. Agent profiles for coobie and sable
get `dispatch:` blocks.

### Phase 2 addition

Wire `FailureDiagnosis` → `CodexPlanAgent` for WrongAnswer failures in Mason's
fix loop. This is also when the `context_paths` restriction is validated: Codex
should only see the staged workspace and test directory, not the full src/.

### Phase 5b

When `src/memory.rs` is split into `src/memory/`, migrate
`BriefingConstruction` to use the new `src/memory/briefing.rs` module as the
sub-agent's primary tool rather than raw file reads. Same dispatch interface;
better retrieval quality.

### Phase 8

`EpisodeCapture` and `Calvin Archive` queries become sub-agent tasks so the
heavy TypeDB graph traversal never appears in the orchestrator's main context.
`BriefingConstruction` routes through the Calvin Archive query surface rather
than flat file retrieval.

---

## Done when (Phase 5-C)

- `SubAgentDispatcher` struct in `src/subagent.rs` with `dispatch()` method
- `[sub_agents]` section parsed from harkonnen.toml into `SetupConfig`
- `coobie_briefing` and `sable_evaluation` tasks dispatch to `ClaudeCodeAgent`
  backend; `DirectLlm` is the fallback for all other tasks
- Agent profile `dispatch:` block parsed and takes priority over global config
- Orchestrator Sable phase uses `dispatcher.dispatch(ScenarioEvaluation, ...)`;
  isolation warnings from sub-agent result logged if non-empty
- `SubAgentResult` fields (`backend_used`, `tokens_used`, `duration_ms`) appended
  to the run's `agent_traces` table for cost and performance observability
- All existing tests pass (DirectLlm backend is behavioral no-op vs current calls)
