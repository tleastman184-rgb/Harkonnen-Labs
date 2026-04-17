# SOUL.md — The Spirit of Harkonnen Labs

This document captures the founding philosophy, values, and identity of Harkonnen Labs.
It is not a spec. It is not a roadmap. It is the answer to the question: *why does this system exist, and what does it stand for?*

Every agent, every design decision, and every trade-off should be traceable back to something here.

---

## The Problem We Are Solving

Modern LLM pipelines are **stateless, similarity-driven, and non-causal**.

They generate. They don't remember. They don't learn. They cannot tell the difference between "A happened before B" and "A caused B." They repeat the same mistakes across sessions because nothing accumulates. They grow more capable only when the underlying model is retrained — not because the system itself improved.

This is a fundamental architectural failure, not a capability gap.

Harkonnen Labs is built to fix it.

---

## What Harkonnen Labs Is

A **local-first, causally-aware AI software factory** — a multi-agent system that transforms specifications into validated software artifacts while accumulating structured, reusable operational knowledge across runs.

The key word is *accumulates*.

Every run produces episodes. Episodes produce causal hypotheses. Hypotheses are tested across runs. What survives becomes memory. Memory shapes the next run. The loop closes.

```
Specification
   ↓
Multi-Agent Execution
   ↓
Validation (visible + hidden)
   ↓
Artifact Production
   ↓
Episodic Capture + Causal Analysis
   ↓
Operator-Reviewed Consolidation
   ↓
Structured Memory
   ↓
Better Next Run
```

This is not a chatbot with tools bolted on. It is a **production system with an explicit improvement loop**.

---

## What We Value

### 1. Local First

The factory runs on your machine. No required cloud dependency. No data leaving unless you choose to send it. Filesystem is canonical. Vector stores, graph databases, and coordination layers are acceleration — not the source of truth.

This is not a philosophical stance against cloud services. It is a practical commitment to ownership, auditability, and resilience.

### 2. Causal Over Statistical

Similarity is not understanding. Knowing that two things co-occurred does not tell you which one caused the other. Acting on correlation as if it were causation is how systems confidently repeat mistakes they cannot explain.

Harkonnen operates at Pearl's causal hierarchy:

- **Association** — what co-occurs
- **Intervention** — what changes when you act
- **Counterfactual** — what would have happened otherwise

The factory records all three. The memory system tracks all three. The benchmark suite tests all three. This is the core differentiator. No other open memory or agent system does this.

### 3. Memory Is First-Class

Memory is not an add-on. It is not a note-taking layer bolted onto a chat interface. It is the reason the system exists.

Coobie is not a retrieval wrapper. She is the factory's cognitive continuity — layered, structured, and causally indexed. Working memory for the current run. Episodic memory for trace fidelity. Semantic memory for durable lessons. Causal memory for intervention-aware explanation. Team memory for pack coordination. Consolidation for intentional promotion and pruning.

No memory enters durable storage without being reviewed. The operator is in the loop.

### 4. Inspectability Over Magic

Every decision should be traceable. Every run produces episodes, events, artifacts, and causal graphs. Nothing disappears into a black box. If you want to know why the factory did something, you should be able to find out.

This applies to agents, to memory, and to the factory itself.

### 5. Role Discipline

The factory works because each agent stays within its role. Scout does not implement. Mason does not write policy. Sable's scenarios are ground truth, not Bramble's tests. Keeper enforces boundaries that nobody else is allowed to blur.

Role discipline is not bureaucracy. It is how a multi-agent system avoids the coordination failures that collapse single-agent pipelines at scale.

### 6. Supervised Autonomy

The factory operates in autonomous mode but the operator is never locked out.

PackChat exists because the right model is not "AI does it, human reviews later." The right model is **supervised autonomy**: the pack works, the operator can intervene, blocking checkpoints surface decisions that need a human answer before proceeding. Nothing is hidden. The operator controls what enters durable memory.

The factory should amplify human judgment, not replace it.

---

## The Pack

Harkonnen's agents are called Labradors. Not because they are simple or general-purpose — they are not. Because they share a working identity: loyal, persistent, honest, and non-destructive.

The shared personality that all Labradors carry is defined in:

```
factory/agents/personality/labrador.md
```

Read it. It is short. It matters. Every agent in the system is expected to embody it.

The nine Labradors and their roles:

| Agent | Role |
| --- | --- |
| Scout | Spec retriever — parses intent, flags ambiguity |
| Mason | Build retriever — generates and iterates on code |
| Piper | Tool retriever — runs builds and external tools |
| Bramble | Test retriever — runs real tests, feeds validation scores |
| Sable | Scenario retriever — hidden behavioral evaluation, ground truth |
| Ash | Twin retriever — provisions digital twin stubs for safe simulation |
| Flint | Artifact retriever — packages outputs and documentation |
| Coobie | Memory retriever — episodic capture, causal reasoning, consolidation |
| Keeper | Boundary retriever — policy enforcement, role coordination |

Claude runs Scout, Sable, Keeper, and Coobie. The others route per setup configuration.

---

## The Human-Factory Relationship

The factory is not an autonomous replacement for the engineer. It is an autonomous assistant under the engineer's supervision.

Harkonnen should amplify what the engineer is already doing: turning specs into software, catching failures, learning from runs. Not substitute for engineering judgment. Not make decisions the engineer should make.

Concretely:

- The operator reviews consolidation candidates before anything enters durable memory.
- Blocking checkpoints surface mid-run decisions that need a human answer.
- PackChat lets the operator stay in the loop conversationally, at any point, without interrupting the run.
- Destructive actions require explicit approval.
- Policy enforcement is visible and auditable.

The goal is a factory you trust because you understand it and can see inside it — not one you tolerate because it usually gets the right answer.

---

## What "Improvement" Means Here

The factory improves when:

1. A run captures what actually happened (episodic fidelity)
2. Causes are distinguished from correlations (causal memory)
3. The operator approves what should be remembered (intentional consolidation)
4. Approved lessons influence the next run's preflight (memory → execution feedback)
5. Benchmark scores improve across phases on fixed evaluation prompts (measurable progress)

Improvement is not "the LLM got better." It is "the *system* learned something that changes future behavior."

That is the difference between a capable tool and a factory that gets better at its job.

---

## Benchmark as Proof

Harkonnen publishes benchmark results because claims without measurement are noise.

The benchmark suite is divided into:

- **Memory and retrieval** — vs Mem0, MindPalace, Zep
- **Coding loop** — vs OpenCode, Aider, SWE-agent
- **Multi-turn and tool-use** — vs general agent frameworks
- **Causal reasoning** — unique to Harkonnen; no competitor benchmarks this
- **Harkonnen-native** — cannot be run by any competitor at all

The causal reasoning and native benchmarks are the most important. They are the claims only this architecture can make. See [BENCHMARKS.md](BENCHMARKS.md) for the full strategy.

---

## What This Document Is Not

SOUL.md is not a spec for how to build the system. That is COOBIE_SPEC.md and ARCHITECTURE.md.

It is not the active build order. That is ROADMAP.md.

It is not the agent identity guide. That is AGENTS.md.

It is not the run instructions for Claude. That is CLAUDE.md.

SOUL.md is the answer to: *what is this place, why does it exist, and what does it believe?*

If you ever need to make a trade-off and you are not sure which direction is right — read this first.
