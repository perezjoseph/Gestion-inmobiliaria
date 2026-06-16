# Harness Patterns

Consolidated from Anthropic, Fowler/Böckeler, OpenAI, and Addy Osmani / Viv Trivedy harness engineering literature.

## First Principles

These govern when and why to apply every pattern below. Without them, the patterns are a checklist; with them, they're a method.

- **Agent = Model + Harness.** If you're not the model, you're the harness. Capability is fixed per model; reliability is what the harness produces. A decent model with a great harness beats a great model with a bad harness.
- **The Ratchet.** Add a constraint only after a real failure. Remove it only when a stronger model made it redundant. Every line in AGENTS.md / a system prompt should trace to a specific failure. This is why a harness can't be downloaded — it's shaped by your failure history.
- **Behavior-derivation.** Each component exists to deliver one named behavior the model can't deliver alone. If you can't name the behavior, the component shouldn't be there.
- **Reliability-per-cost.** Every control costs context budget, maintenance, or latency. A control is good only if it prevents more than it costs. Optimize the ratio, not maximum reliability and not minimum cost.
- **Success silent, failure verbose.** Sensors say nothing on pass; inject actionable error text (what + why + how to fix) on fail. Makes the feedback loop nearly free in the common case.
- **Harnesses move, don't shrink.** A stronger model kills some scaffolding (e.g. context-anxiety mitigation) but unlocks tasks needing new scaffolding (multi-day memory, multi-agent coordination, design-quality evaluators). The ceiling moves with the model. Every component encodes an assumption about what the model can't do alone — when that stops being true, remove it.

## What Makes a Harness Good

A good harness is the minimal set of mechanical controls that makes the agent's failure modes impossible rather than improbable, scoped tightly enough that sensors cover the whole action space, recoverable after every session, and cheap enough that each control prevents more than it costs.

It reduces to three properties — every pattern below serves one of them:

1. **Detectability** — no failure the harness cares about can happen silently. Move correctness from "the agent chose to" into "the system enforced."
2. **Bounded variety** — narrow the action space (role, WIP=1, denied paths, minimal tools) until a finite sensor set covers it. The hole reliability leaks through is any capability no sensor checks.
3. **Recoverability** — the agent can always reconstruct "where am I, what's verified, what's next" from artifacts. No session can leave an unresumable state.

## Generator/Evaluator

Agent generates, sensors evaluate. Never self-evaluate.

- Agents skew positive grading own work. Separate generation from evaluation.
- Evaluator = computational sensors (fmt, clippy, tests). Deterministic, objective.
- Run 1–5 iterations: evaluator findings feed back as generator input.
- Tuning inferential evaluators: read logs, find divergence from your judgment, update prompt. Repeat.

For computational evaluators (this repo's hooks), tuning isn't needed — they're deterministic.

## Feedforward + Feedback

Both required. One without the other fails.

| Type | Role | Examples |
|------|------|----------|
| Feedforward (guides) | Steer BEFORE agent acts | AGENTS.md, steering files, skill instructions, type system |
| Feedback (sensors) | Observe AFTER agent acts, enable correction | Linters, tests, type-checkers, custom structural tests |

**Computational** sensors: deterministic, fast, cheap. Run on every change.
**Inferential** sensors: LLM-as-judge, semantic review. Slower, non-deterministic. Cannot replace computational checks.

Powerful when sensor output is optimized for LLM consumption (custom lint messages with fix instructions baked in).

## The Steering Loop

Recurring issue → improve harness, not individual fix.

1. Improve feedforward to make issue less probable
2. Improve sensors to catch it earlier
3. Use AI to build custom controls (structural tests, linters)

> "Every time agent makes a mistake, make that failure impossible by design."

## Keep Quality Left

Cheapest sensors first. Fail early.

```
Format (< 1s) → Lint (5-15s) → Compile (15-30s) → Test (30-120s) → E2E (minutes)
```

## Ashby's Law / Constrain Variety

LLM can produce anything. Committing to a topology (Rust workspace, TS sidecar, K8s manifests) narrows output space → comprehensive harness becomes achievable.

Pre-defined sensor suites per topology make governance tractable. This repo's topology: Actix-web backend, Leptos frontend, Kotlin Android, Node sidecar.

## Regulation Categories

| Category | What it checks | Sensor examples |
|----------|---------------|-----------------|
| Maintainability | Internal code quality | Complexity, coverage, style, duplication |
| Architecture | Fitness functions | Layer boundaries, perf requirements, module deps |
| Behavior | Functional correctness | Tests, specs, manual QA (hardest to automate) |

## Repo as Source of Truth

If agent can't see it in-repo, it doesn't exist.

- AGENTS.md = table of contents (~100 lines), not encyclopedia
- Monolithic instruction files fail: crowd context, rot fast, can't verify mechanically
- Encode decisions, conventions, architecture as versioned repo artifacts
- Slack/docs/tacit knowledge = invisible to agent until committed

## Progressive Disclosure

```
Tier 1 (always loaded): metadata, feature/task status, tech stack
Tier 2 (on activation): skill bodies, steering files, style guides
Tier 3 (on demand): architecture docs, API refs, schemas
```

Kiro mapping: Tier 1 = `inclusion: always`, Tier 2 = `inclusion: fileMatch` + skill activation, Tier 3 = `inclusion: manual` + sub-agents.

## Agent Legibility

Optimize for agent reasoning, not just human readability:

- Rigid layered architecture with validated dependency directions
- Custom lint errors inject remediation instructions into agent context
- Boring tech with composable APIs and training-set representation
- Sometimes cheaper to reimplement than fight opaque upstream

## Architecture Enforcement

Strict boundaries = prerequisite for speed without decay.

- Fixed layer ordering per domain
- Limited permissible dependency edges
- Custom linters + structural tests (agent-generated)
- Lint messages teach the agent how to fix

## Entropy and Garbage Collection

Agent replicates existing patterns — including bad ones. Drift inevitable.

- Encode "golden principles" into repo
- Continuous small cleanup, not periodic slop days
- Recurring tasks scan for deviations, open targeted fix PRs
- Human taste captured once, enforced continuously

## Context Management

Context is a budget, not a dump.

| Operation | What | When |
|-----------|------|------|
| SELECT | Load just-in-time | Feature starts, skill activates |
| WRITE | Agent writes back to persistent storage | Progress, discovered rules |
| COMPRESS | Compact older turns | Context > 80% |
| ISOLATE | Delegated work can't pollute parent | Sub-agents, parallel tasks |

Isolation patterns:
- Coordinator (zero inheritance): workers start fresh
- Fork (full, single-level only): quick parallel splits
- Swarm (shared task list): long-running independent work

## Sprint Contracts

Before building, agree on "done":
- Generator proposes what it will build + how success is verified
- Evaluator confirms correctness criteria are testable
- In CI autofix: contract = set of sensors that must pass

## Simplify as Models Improve

Every harness component encodes an assumption about what the model can't do alone. Assumptions go stale.

When new model lands: strip pieces no longer load-bearing. Add new pieces for greater capability. The space moves, doesn't shrink.

**Practice:** Monthly, disable one component, run benchmark tasks. No degradation → remove. Degradation → keep or replace with lighter alternative.

**Example (Anthropic):** Sprint-splitting mechanism essential for Sonnet 4.5 became unnecessary overhead with Opus 4.6 (model handles decomposition natively). But evaluator still needed even with Opus 4.6 at capability boundaries.

## Fresh Session Test

Five questions a brand-new session must answer from repo contents alone:
1. What is this system?
2. How is it organized?
3. How do I run it?
4. How do I verify it?
5. Where are we now (progress)?

If any unanswerable → harness has a gap. Run this test periodically.

## ACID for Agent State

- **Atomicity:** One git commit per logical operation. Fails midway → stash/rollback.
- **Consistency:** Verification predicates define "consistent state." Never commit inconsistent.
- **Isolation:** Multiple agents → own progress files or branches. No concurrent writes to same file.
- **Durability:** Cross-session knowledge in git-tracked files. What's in memory doesn't survive.

## Instruction Hygiene

- **SNR (Signal-to-Noise):** Proportion of loaded instructions relevant to current task. 600-line file for a bug fix = low SNR.
- **Lost in the Middle:** LLMs utilize middle of long text significantly less (Liu et al. 2023). Critical rules at position 300/600 → frequently ignored. Put at top or bottom.
- **Rule lifecycle:** Every instruction needs source (why added), applicability (when needed), expiry (when removable).
- **Knowledge Decay Rate:** Stale docs worse than no docs. Must update with code.

## Context Anxiety

Agents rush to finish when context fills. Skip verification, choose simple over optimal.

- Sonnet 4.5: severely affected → context resets necessary
- Opus 4.5/4.6: greatly diminished → compaction sufficient

**Implication:** Harness design must be model-aware. Strategies that work for one model may not for another.

## Feature List as Harness Primitive

Not a memo. It's the foundational data structure the scheduler, verifier, and handoff reporter all depend on.

**Triple structure:** `(behavior_description, verification_command, current_state)`

**States:** `not_started` → `active` → `passing` (or `blocked`). Only verification command success allows transition to `passing`. Irreversible.

**Granularity:** Completable in one session. "Add user auth" = too broad. "POST /register returns 201" = right size.

**Data:** Structured feature lists → 45% higher completion rate, zero duplicate implementations vs free-form tracking.

## Review Feedback Promotion

Recurring code review comment → automated check. Each promotion makes harness permanently stronger.

Process:
1. Identify recurring issue category
2. Write automated check (lint rule, structural test)
3. Write agent-oriented error message (what + why + how to fix)
4. Add to harness
5. Issue never recurs

## Observability (Two Layers)

| Layer | What | Answers |
|-------|------|---------|
| Runtime | Logs, traces, health checks, process events | "What did the system do?" |
| Process | Plans, sprint contracts, scoring rubrics, acceptance criteria | "Why should this be accepted?" |

Both required. Missing either → 30-50% of session time wasted on redundant diagnosis.

## Clean State (Five Dimensions)

Session isn't "done" until all pass:
1. Build passes
2. Tests pass
3. Progress recorded (machine-readable)
4. No stale artifacts (debug logs, temp files, TODO markers)
5. Standard startup path works

**Data (12-week comparison):**
- Without cleanup: build 68%, tests 61%, startup 60+ min
- With cleanup: build 97%, tests 95%, startup 9 min

## Diagnostic Loop (Five-Layer Attribution)

When agent fails, don't say "model bad." Attribute to one of five layers:

1. **Task specification** — was it clearly defined?
2. **Context provision** — did the agent have enough information?
3. **Execution environment** — was the environment reproducible?
4. **Verification feedback** — were sensors present?
5. **State management** — was cross-session continuity maintained?

Fix the identified layer. Re-execute. Repeat. After several rounds, harness strengthens and failures stop recurring in that layer.

**Audit method (Controlled Variable Exclusion):** Keep model fixed, remove one subsystem at a time, measure performance drop. Largest drop = highest marginal value for current task. But to find the actual bottleneck, also examine failure logs and do root-cause attribution — ablation alone isn't proof.

## WIP=1 (Scope Constraint)

Agents activating multiple tasks simultaneously → none finish well. Math: context capacity C split across k tasks gives C/k reasoning per task. When C/k < minimum threshold, nothing completes.

**Data:** Agents with WIP=1 strategy show 37% higher completion rate than broad-prompt agents. Lines of code generated negatively correlates with features completed.

Enforce in AGENTS.md / steering:
- Only one task in "active" status at any time
- Don't start next until current passes E2E verification
- Don't "also refactor" unrelated code while implementing a feature

## Three-Layer Termination Check

Agent declaring "done" ≠ actually done. Confidence calibration bias = systematic overconfidence (proven by Guo et al. 2017 — model confidence significantly exceeds actual accuracy).

Three layers, sequential:
1. **Syntax/static** — lint, typecheck, format. Cheapest, must pass first.
2. **Runtime behavior** — tests pass, app starts, critical paths execute.
3. **System-level** — E2E flow, integration, user scenario simulation.

No skipping layers. No declaring done at layer 1.

**Completion Priority Constraint:** Verify functional correctness → then performance → then style. No refactoring until core functionality passes verification.

## Harnessability

Not every codebase equally amenable:
- Strongly typed → type-checking as free sensor
- Clear module boundaries → constraint rules possible
- This repo: Rust types + Actix extractors + SeaORM = high harnessability

## Pi Patterns (Lifecycle Hooks as Harness Primitives)

Pi (pi.dev) implements harness engineering through a typed event/hook system. Key patterns applicable to any agent harness:

### Event-Driven Tool Gating

```
tool_call event → can BLOCK before execution
tool_result event → can MODIFY after execution
```

Kiro equivalent: `preToolUse` hooks block/allow, `postToolUse` hooks run sensors. The pattern: intercept at the boundary, not inside the tool.

### Structured Compaction (not just "summarize old stuff")

Pi's compaction preserves:
- Goal (what user is trying to accomplish)
- Constraints & preferences
- Progress (done / in-progress / blocked)
- Key decisions with rationale
- Next steps
- Read files and modified files (cumulative across compactions)

This is the WRITE operation from context engineering — agent writes structured state back before context is pruned.

### Session Branching as Exploration

Pi treats sessions as trees with fork/clone. When abandoning a branch, it summarizes work done on that branch and injects it into the new branch. Pattern: **exploration doesn't lose context — it gets summarized and carried forward.**

### Extension-Based Harness Composition

Pi's harness isn't monolithic — it's composed from independent extensions that each own one concern:
- Permission gate extension (blocks dangerous commands)
- Git checkpoint extension (stashes at each turn)
- Path protection extension (blocks writes to sensitive files)
- Custom compaction extension (summarizes your way)

Kiro equivalent: each `.kiro/hooks/*.kiro.hook` file owns one harness concern. Composable, removable, debuggable independently.

### Progressive Tool Activation

Pi supports `setActiveTools()` — dynamically enable/disable tools based on task phase. Pattern: don't give agent all tools at once. Narrow the action space to what's relevant now.

Kiro equivalent: sub-agents with restricted tool sets. Each agent sees only tools relevant to its domain.

### Output Truncation as Feedback Design

Pi enforces 50KB / 2000 line limits on tool output. When truncated, tells the agent WHERE to find full output. Pattern: **feedback must be sized for LLM consumption.** A 10MB test log is noise, not signal. Truncate + pointer.

### File Mutation Queue

Pi queues writes to the same file — parallel tool calls can't clobber each other. Pattern: **when agent throughput exceeds sequential execution, add coordination primitives.** Relevant for multi-agent setups where two sub-agents might edit the same file.

## Anti-Patterns

1. Self-evaluation without sensors — agent says "looks good" even when broken
2. One giant instruction file — crowds out task context, rots fast
3. No state persistence — each session starts from zero
4. No scope boundaries — half-finishes three things
5. No init phase — starts on broken foundation
6. Invisible knowledge — decisions in Slack/docs never reach agent
7. Incremental patching — same error twice = wrong approach, step back
8. Isolated artifacts — config without system prompt is incomplete
9. Individual resource listings — use globs, individual paths rot
