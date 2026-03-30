For the implementation-facing module, trait, and TypeDB plan, see [COOBIE_SPEC.md](./COOBIE_SPEC.md).

# Coobie Memory Architecture

Coobie is the local memory engineer for Harkonnen Labs.

Her job is not just to fetch notes. She manages three memory layers so the Labrador pack can stay coherent across a run, across sessions, and across a multi-agent workflow.

## Design Goal

Keep the entire memory system local-first.

Source of truth stays on disk inside this repository. Optional local services accelerate retrieval and coordination, but they do not become the canonical record.

## Memory Layers

### 1. Short-Term Memory

Purpose: keep one run or thread coherent.

This layer holds:

- current run state
- recent tool outputs
- recent agent outputs
- current implementation plan
- current validation state
- current team board snapshot

Recommended local backing:

- SQLite for structured run state
- per-run files under `factory/workspaces/<run-id>/run/`
- optional Redis later for hot coordination and transient team state

Why not raw prompt history:

- token growth gets expensive quickly
- it becomes harder to debug what mattered
- different agents start carrying different partial histories

### 2. Long-Term Memory

Purpose: remember what should still matter next week.

This layer holds:

- architecture notes
- imported documents
- imported PDFs and images
- past failures
- setup decisions
- org-specific constraints
- run reflections
- reusable implementation patterns

Canonical local source of truth:

- `factory/memory/notes/` or top-level memory markdown
- `factory/memory/imports/` for imported assets
- `factory/memory/reflections/` for consolidated run learning
- `factory/memory/index.json` as the local search index

Current implementation status:

- local markdown and asset import are live
- local auto-refresh indexing is live
- AnythingLLM sync is available as an optional local accelerator

Recommended local semantic accelerator:

- Qdrant

Qdrant should be used as a semantic index over extracted text and memory summaries, not as the source of truth. Payload metadata should include:

- `org`
- `role`
- `product`
- `spec_id`
- `run_id`
- `agent`
- `memory_type`
- `tags`
- `created_at`

### 3. Team Memory

Purpose: keep the full Labrador pack aligned on shared state.

This layer holds:

- current run objective
- shared plan
- claimed tasks
- intermediate artifacts
- blocked states
- handoff notes
- decision log

Recommended local backing:

- phase 1: SQLite or per-run `board.json`
- phase 2: optional Redis for hot shared state and low-latency coordination

Redis is a fit here because team memory is mutable, hot, and coordination-heavy. It is not the best source of durable memory.

## Service Roles

### Filesystem

Role: authoritative memory source.

Stores:

- markdown notes
- imported assets
- run reflections
- extracted text sidecars

### SQLite

Role: structured short-term and team memory.

Stores:

- run metadata
- event logs
- team board state
- agent claims and checkpoints

### Qdrant

Role: long-term semantic memory index.

Use Qdrant for:

- semantic retrieval over extracted document text
- memory filtering by payload metadata
- retrieving similar failures, setup notes, and prior runs

Do not use Qdrant as the only place memory lives.

### Redis

Role: optional hot memory and coordination layer.

Use Redis for:

- active team board cache
- transient short-term session buffers
- run-local locks
- work claims and queue state

Do not use Redis as the only durable memory store.

### AnythingLLM

Role: optional local retrieval surface.

Use AnythingLLM for:

- local RAG over imported documents
- richer file ingestion
- local model-backed document query flows

AnythingLLM should accelerate retrieval, not replace the local filesystem memory tree.

## Recommended Local Build Order

### Phase A: Files First

Build and rely on:

- local markdown memory
- asset import into `factory/memory/imports/`
- auto-refresh local index
- manual and CLI memory search

Status: implemented.

### Phase B: Extraction

Add local extractors so imported assets become more searchable locally.

Recommended tools:

- `poppler-utils` for `pdftotext`
- `tesseract-ocr` for OCR on images and scanned PDFs
- optional `libreoffice --headless` or `pandoc` for office docs

Result:

- Coobie gets real extracted text sidecars
- Qdrant and AnythingLLM both get better input

### Phase C: Qdrant

Add local Qdrant as Coobie's semantic long-term memory index.

Use it for:

- nearest-neighbor retrieval
- filtered semantic recall
- org- and run-scoped memory lookups

Status: scaffold scripts added in `scripts/bootstrap-coobie-memory-stack.sh`.

### Phase D: Team Board

Add a structured team memory board.

Start with:

- SQLite or `board.json`

Later optionally promote hot shared state into Redis when concurrency or Pack Board live updates justify it.

### Phase E: Reflection and Forgetting

Add a consolidation pass after each run.

Responsibilities:

- compress repetitive run logs into durable summaries
- store reusable procedural lessons
- decay or archive low-value episodic events
- keep the long-term store lean

## Coobie Responsibilities

Coobie should explicitly own:

- short-term context retrieval for the current run
- long-term retrieval over notes, assets, and reflections
- team board reads for cross-agent coordination
- post-run consolidation into durable memory
- memory hygiene, indexing, and retrieval quality

## Operational Commands

Initialize memory:

```bash
cargo run -- memory init
```

Import a document, PDF, or image asset:

```bash
cargo run -- memory import /path/to/file.pdf --summary "Factory setup guide" --tags setup,docs
```

Search Coobie's local memory:

```bash
cargo run -- memory search setup
```

Sync local memory into AnythingLLM:

```bash
export ANYTHINGLLM_API_KEY=...
export ANYTHINGLLM_BASE_URL=http://localhost:3101/api
./scripts/coobie-seed-anythingllm.sh
```

Write local Qdrant and Redis launch scaffolding:

```bash
./scripts/bootstrap-coobie-memory-stack.sh
```

## Current Repo Mapping

- `src/memory.rs`
- `src/cli.rs`
- `scripts/coobie-seed-anythingllm.sh`
- `scripts/coobie-seed-mcp.sh`
- `factory/agents/profiles/coobie.yaml`

## Final Rule

For Harkonnen Labs, memory stays trustworthy only if local files remain the source of truth.

- Files are canonical.
- Qdrant is semantic acceleration.
- Redis is hot coordination.
- AnythingLLM is a local retrieval surface.
