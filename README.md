# Harkonnen Labs

## A Local-First, Causally-Aware AI Software Factory (WIP)

Harkonnen Labs is a **multi-agent software execution system** that transforms specifications into validated software artifacts while accumulating **structured operational knowledge** across runs.

At its core, Harkonnen is designed to solve a specific failure mode in modern LLM systems:

> LLM pipelines are *stateless, similarity-driven, and non-causal* — they cannot reliably improve from experience.

Harkonnen introduces a **stateful, causally-informed execution model** where:

* **Agents** perform bounded roles in a production pipeline
* **Memory** persists across runs with explicit structure
* **Causal reasoning** separates correlation from intervention
* **Policy** governs what actions are allowed, not just what is possible

The result is a system that does not just generate software — it **learns how to produce better software over time**.

---

## Conceptual Model

Harkonnen operates as a **closed-loop software factory**:

```text
Specification
   ↓
Multi-Agent Execution
   ↓
Validation (including hidden scenarios)
   ↓
Artifact Production
   ↓
Memory Ingestion (episodic)
   ↓
Consolidation → Semantic + Causal Knowledge
   ↓
Improved Future Execution
```

This loop converts **execution traces into reusable knowledge**.

---

## System Components

### 1. Agent Pack (Execution Layer)

Harkonnen decomposes execution into nine specialist agents:

| Agent | Role | What it does |
| --- | --- | --- |
| **Scout** | Spec retriever | Parses specs, flags ambiguity, produces normalized intent packages |
| **Mason** | Build retriever | Generates and modifies code; iterates up to 3 times on build failure with structured fix-loop reasoning |
| **Piper** | Tool retriever | Runs build tools, fetches docs, executes helpers with live stdout/stderr streaming |
| **Bramble** | Test retriever | Generates visible tests, runs lint and build, feeds pass/fail into Coobie scoring |
| **Sable** | Scenario retriever | Executes hidden behavioral scenarios; its results are the ground truth, not Bramble's |
| **Ash** | Twin retriever | Provisions digital twin manifests and dependency stubs for safe external-system simulation |
| **Flint** | Artifact retriever | Collects outputs, packages artifact bundles for inspection and evidence |
| **Coobie** | Memory retriever | Manages layered memory — episodic capture, causal reasoning, Palace patrol, and consolidation |
| **Keeper** | Boundary retriever | Enforces policy, guards role boundaries, owns file-claim coordination |

This is not a monolithic agent — it is a **role-constrained system with explicit handoffs**.
Scout, Sable, and Keeper are pinned to Claude. All others route per setup configuration.

---

### 2. Coobie (Layered Memory System)

Coobie implements a **multi-layer memory architecture** that goes beyond a flat note store:

| Layer | Purpose | Status |
| --- | --- | --- |
| Working Memory | Current run state (compressed, ephemeral, token-budgeted) | Live |
| Episodic Memory | Ordered execution traces (state → action → result) with phase attribution | Live |
| Semantic Memory | Stable facts, patterns, invariants — hybrid vector + keyword retrieval | Live |
| Causal Memory | Intervention-aware cause/effect relationships scored per run | Live |
| Team Blackboard | Shared agent coordination state across four named slices | Live |
| Consolidation | Promotion, pruning, and abstraction (operator-reviewed in Phase 5) | Partial |

#### Coobie Palace

The Palace is a **spatially-organized compound recall layer** built on top of Coobie's causal memory. Related failure causes are grouped into named **Dens** that Coobie patrols before every run:

| Den | Residents | What it covers |
| --- | --- | --- |
| Spec Den | `SPEC_AMBIGUITY`, `BROAD_SCOPE` | Failures from unclear or over-scoped specs |
| Test Den | `TEST_BLIND_SPOT` | Visible tests passed but hidden scenarios caught failures |
| Twin Den | `TWIN_GAP` | Simulated environment didn't match production |
| Pack Den | `PACK_BREAKDOWN` | Degraded or incomplete Labrador phase execution |
| Memory Den | `NO_PRIOR_MEMORY` | Factory ran cold with no relevant prior context |

A **Patrol** walks each den before a run, computing a compound **Scent** — when multiple causes from the same den have fired together, the whole den is elevated, not just the individual signal. Results inject directly into the preflight briefing's `required_checks`, `guardrails`, and `open_questions`. The principle: *the whole den smells, not just one corner*.

#### Causal Reasoning

Coobie tracks three levels of the causal hierarchy:

* **Association** — co-occurrence patterns across runs
* **Intervention** — outcome changes due to explicit actions, recorded in the causal link table
* **Counterfactuals** — inferred alternative outcomes

Live features:

* Causal streaks — a cause that fires repeatedly across runs is escalated in the preflight briefing
* Cross-run pattern detection — identifies causes that cluster on specific spec types or phases
* Phase 3 preflight guidance — spec-scoped cause history drives `required_checks` before each run
* Palace compound recall — den-level streak weight elevates compound failures beyond individual cause scores
* Causal feedback loop — Sable's scenario rationale is written back to project memory as evidence

---

### 3. PackChat — Conversational Control Plane

PackChat shifts the factory from pure autonomous orchestration to **supervised autonomy** — you stay in the loop through the same conversation surface while the pack works.

* **Chat threads** scoped to a run or spec, persisted in SQLite
* **`@mention` routing** — `@coobie what did we learn?`, `@mason explain the fix` — dispatches to the right Labrador with its full system role loaded
* **Blocking checkpoint flow** — when an agent needs an answer before proceeding, it surfaces a structured reply card in the thread; you answer there, the run continues
* **Unblock flow** — `POST /api/agents/:id/unblock` releases a stalled run after you reply
* **Default to Coobie** for unaddressed messages — memory and context retrieval without `@mention`

---

### 4. Persistence Model

* **SQLite** → episodic memory, run state, chat threads, phase attributions, causal links
* **Filesystem** → specs, artifacts, evidence, Coobie memory (`factory/memory/*.md`)
* **fastembed / OpenAI-compatible embeddings + SQLite vector store** → hybrid semantic + keyword retrieval (live)
* **(Planned / Optional) TypeDB 3.x service** → durable semantic graph + typed relational queries, not the hot-path store
* **(Future) Causal graph / causaloids** → executable causal reasoning via DeepCausality

---

### 5. Benchmarking And Improvement Loop

Harkonnen now includes a first-class benchmark toolchain so changes can be measured, compared, and regressed automatically.

Core entrypoints:

```bash
cargo run -- benchmark list
cargo run -- benchmark run
cargo run -- benchmark run --suite local_regression --strict
cargo run -- benchmark run --all
./scripts/run-benchmarks.sh
```

The machine-readable suite manifest lives at `factory/benchmarks/suites.yaml`, benchmark strategy and reporting guidance live in `BENCHMARKS.md`, and reports are written to `factory/artifacts/benchmarks/`. The default suite is a local regression gate, LongMemEval and LoCoMo now both run in native Harkonnen and raw-LLM baseline modes, and the remaining external adapters are prepared for tau2-bench and SWE-bench Verified/Pro. The execution roadmap in `ROADMAP.md` now treats benchmark gates as phase-level exit criteria rather than optional follow-up work.

The OpenAI/Codex provider path also supports optional OpenAI-compatible BYO endpoints through a setup `base_url`, so benchmark runs can target local or third-party compatible backends without changing Rust code.

### Benchmark Results

Status legend: **wired** = adapter integrated and runnable; **planned** = adapter not yet built; **internal** = Harkonnen-native, no external suite.

All scores are pending first runs. The comparison targets listed are the systems each benchmark is designed to compare against.

#### Memory and retrieval — vs Mem0 / MindPalace / Zep

| Benchmark | Subsystem | Metric | Harkonnen | Raw LLM baseline | Comparison target | Status | Phase |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| LongMemEval-S | Coobie | Accuracy | pending | pending | Mem0 / raw LLM | wired | Phase 1 done |
| FRAMES | Coobie | Multi-hop accuracy | pending | pending | Mem0 / raw LLM | planned | Phase 4 |
| StreamingQA | Coobie | Belief-update accuracy | pending | pending | raw LLM | planned | Phase 4 |
| LoCoMo QA | Coobie | QA score | pending | pending | raw LLM | wired | Phase 1 done |
| HELMET | Coobie | Retrieval precision / recall | pending | pending | raw LLM | planned | Phase 4 |

#### Causal reasoning — unique to Harkonnen

| Benchmark | Subsystem | Metric | Harkonnen | Raw LLM baseline | Notes | Status | Phase |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| CLADDER | Coobie / causal layer | Accuracy by causal level | pending | pending | No competitor publishes this | planned | Phase 4 |
| E-CARE | Coobie / diagnose | Explanation coherence | pending | pending | Tests diagnose output quality | planned | Phase 5 |

#### Coding loop — vs OpenCode / Aider / SWE-agent

| Benchmark | Subsystem | Metric | Harkonnen | Competitor baseline | Comparison target | Status | Phase |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| SWE-bench Verified | Mason / Piper / Bramble | % Resolved | pending | pending | SWE-agent / OpenCode | planned | Phase 2 |
| SWE-bench Pro | Mason / Piper / Bramble | % Resolved | pending | pending | SWE-agent | planned | Phase 2 |
| LiveCodeBench | Mason / Piper | Pass rate | pending | pending | OpenCode / Aider | planned | Phase 2 |
| Aider Polyglot | Mason / Piper | % Correct | pending | pending | Aider (published leaderboard) | planned | Phase 2 |
| DevBench | Full factory pipeline | Lifecycle score | pending | pending | Single-agent tools | planned | Phase 3 |

#### Multi-turn and tool-use — vs general agent frameworks

| Benchmark | Subsystem | Metric | Harkonnen | Competitor baseline | Comparison target | Status | Phase |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| tau2-bench | PackChat | Pass^1 / Pass^4 | pending | pending | raw LLM | launcher wired | Phase 1+ |
| GAIA Level 3 | Full factory (Scout → Sable) | Task completion | pending | pending | General agent frameworks | planned | Phase 6 |
| AgentBench (OS / DB / web) | Labrador roles | Env pass rate | pending | pending | Single-generalist frameworks | planned | Phase 6 |

#### Harkonnen-native — no competitor can run these

| Benchmark | Subsystem | Metric | Result | Notes | Status | Phase |
| --- | --- | --- | ---: | --- | --- | --- |
| Spec Adherence Rate | Scout / Mason | Completeness % / Precision % | pending | Measures spec-first contribution — run with and without Scout | internal | Phase 3 |
| Hidden Scenario Delta | Bramble / Sable | Pass rate gap (hidden − visible) | pending | Proves Sable catches what Bramble misses | internal | Phase 3 |
| Causal Attribution Accuracy | Coobie diagnose | Top-1 / Top-3 accuracy | pending | Seeded failure corpus; measures causal memory vs semantic recall | internal | Phase 5 |
| Local Regression Gate | Whole factory | pass / fail | passing | Hard merge gate, runs on every change | wired | Phase 1 done |

Full benchmark strategy, adapter environment variables, and reporting guidance: [BENCHMARKS.md](BENCHMARKS.md)

---

### 6. Execution Semantics

Each run produces:

1. **Artifacts** (code, configs, outputs)
2. **Episodes** (what happened)
3. **Evaluations** (did it work?)
4. **Memory updates** (what should be remembered)

Over time:

> The system transitions from *prompt-driven behavior* to *memory-informed behavior*.

---

## ⚡ Quickstart

### 1. Clone + Build

```bash
git clone https://github.com/durinwinter/Harkonnen-Labs.git
cd Harkonnen-Labs

cargo build
```

---

### 2. Start the Factory

```bash
cargo run
```

You should see:

* agent initialization
* memory system startup
* factory ready state

---

### 3. Create a Spec

```bash
mkdir -p specs
```

```json
// specs/hello_api.json
{
  "name": "hello-api",
  "description": "Create a simple REST API",
  "language": "rust",
  "requirements": [
    "axum server",
    "GET /hello endpoint",
    "returns JSON"
  ]
}
```

---

### 4. Run the Spec

```bash
cargo run -- run specs/hello_api.json
```

---

### 5. Inspect Outputs

Artifacts:

```bash
artifacts/
```

Runs:

```bash
runs/<run_id>/
```

Memory:

```bash
factory/memory/
```

---

## 🛠 Core Commands

### Run a spec

```bash
cargo run -- run <spec.json>
```

---

### Run with memory influence

```bash
cargo run -- run <spec.json> --with-memory
```

---

### List runs

```bash
cargo run -- runs list
```

---

### Inspect a run

```bash
cargo run -- runs inspect <run_id>
```

---

### Inspect memory

```bash
cargo run -- memory list
```

```bash
cargo run -- memory inspect <memory_id>
```

---

### Ingest knowledge

```bash
cargo run -- memory ingest ./docs/
```

```bash
cargo run -- memory ingest https://example.com
```

---

### Debug mode

```bash
RUST_LOG=debug cargo run -- run specs/hello_api.json
```

---

##  Project-Level Memory

Each project can maintain isolated memory:

```text
.harkonnen/
  project-memory/
  evidence/
```

This enables:

* per-repo learning
* reuse of patterns
* isolation across domains

---

##  Example Memory Evolution

### Episode

```json
{
  "action": "retry with schema validation",
  "result": "success",
  "context": {
    "language": "rust"
  }
}
```

---

### Semantic Fact

```json
{
  "fact": "schema validation improves structured outputs",
  "confidence": 0.81
}
```

---

### Causal Claim

```json
{
  "claim": "disabling schema validation reduces latency but increases failure rate",
  "confidence": 0.74
}
```

---

##  Execution Loop

```text
Spec → Agents → Validation → Artifacts → Memory → Consolidation → Better Next Run
```

---

##  Design Principles

* **Local-first** — no required cloud dependency
* **Inspectable** — every decision traceable
* **Composable** — agents are modular
* **Causal over statistical** — prefer explanation over similarity
* **Memory is first-class** — not an afterthought

---

## ⚠️ Status

Harkonnen Labs is an **active development system** — Phase 1 backend is shipped.

| Area | Status |
| --- | --- |
| Core factory pipeline (Scout → Mason → Piper → Bramble → Sable → Ash → Flint) | Working |
| Mason fix loop (up to 3 iterations on build failure) | Live |
| PackChat conversational control plane | Live — threads, `@mention` routing, checkpoint/unblock flow |
| Coobie layered memory (episodic, semantic, causal) | Live |
| Coobie Palace (den-based compound recall, patrol, scent) | Live |
| Coobie causal streaks and cross-run pattern detection | Live |
| Coobie preflight guidance (Phase 3 spec-scoped cause history) | Live |
| Hybrid semantic + keyword retrieval (fastembed / OpenAI-compatible) | Live |
| Pack Board React UI (PackChat, Attribution Board, Factory Floor, Memory Board) | Live |
| Keeper coordination API (claims, heartbeats, conflict detection) | Live |
| Benchmark toolchain (manifest-driven, LongMemEval + LoCoMo native adapters) | Live |
| Bramble real test execution | Phase 2 — next |
| Ash live twin provisioning (Docker stubs) | Phase 3 |
| Episodic layer enrichment + causal link graph | Phase 4 |
| Operator consolidation Workbench | Phase 5 |
| TypeDB 3.x semantic graph layer | Phase 6 |

See [ROADMAP.md](ROADMAP.md) for the full phase-by-phase build order.

---

## 🚀 Direction

Near-term (Phase 2–3):

* Bramble real test execution — make `validation_passed` meaningful
* Ash live twin provisioning — ground Sable's scenario evaluation in running stubs
* Flint documentation phase — enable DevBench lifecycle scoring
* FRAMES and CLADDER benchmark adapters — the Mem0 and causal reasoning comparison lines

Mid-term (Phase 4–5):

* Episodic layer enrichment — causal link table, Pearl hierarchy in `diagnose`
* Multi-hop retrieval chain — beat single-pass vector stores on FRAMES
* Memory invalidation tracking — belief-update accuracy (StreamingQA)
* Operator consolidation Workbench — nothing enters durable memory without approval

Long-term (Phase 6+):

* TypeDB 3.x semantic graph — typed causal queries that vector similarity cannot answer
* **Self-improving software factory** — each run makes the next run better


