---
name: harness-engineering
description: >
  Build, audit, and improve agent harnesses: kiro-cli agent configs, CI autofix
  workflows, verify-fix loops, hooks, feedforward/feedback controls, session
  lifecycles, state persistence, context budgets, and scope control. Use when
  asked about generator/evaluator separation, context resets vs compaction,
  Ashby's Law, agent legibility, repo-as-source-of-truth, or progressive
  disclosure for agents. Also use when improving repo harnessability, designing
  multi-agent coordination, or diagnosing why an agent keeps failing a task.
license: MIT
compatibility: Requires Node.js 18+ for scripts/validate-kiro-agent.js
metadata:
  author: perezjoseph
  version: "3.0"
---

# Harness Engineering for Coding Agents

The model decides what code to write. The harness governs when, where, and how. A harness doesn't make the model smarter — it makes output reliable.

## Core Model

Every reliable agent harness has five subsystems. A *mature* harness has all five; a v0.1 does not. Start minimal, get it running, then ratchet up — add a subsystem when a real failure demands it (see Building Sequence below). A gap is only a problem once failures land in it.

| Subsystem | Job | This Repo |
|-----------|-----|-----------|
| Instructions | What to do, in what order, what to read first | `AGENTS.md` (map) + `.kiro/steering/` (progressive disclosure) |
| State | What's done, in progress, next | Kiro specs, git log |
| Verification | Passing sensors = only valid evidence | Hooks (clippy-on-save, harden-app), CI |
| Scope | One task until verified, no overreach | Spec tasks, sub-agent delegation |
| Lifecycle | Clean start, clean end, restartable next session | git-commit-push-on-task-complete hook |

## When Activated — First Move

1. Diagnose which subsystem is weakest (use the decision tree below)
2. Read only the reference needed for that gap
3. Produce complete deliverables — never isolated artifacts

## Diagnosis: Decision Tree

```
Agent produces wrong output?
├── Wrong approach         → Instructions gap (doesn't know HOW)
├── Right approach, wrong scope → Scope gap (doing too much/little)  
├── Says "done" but broken → Verification gap (no sensors)
├── Works once, breaks next session → State gap (no continuity)
└── Drifts over time       → Lifecycle gap (no cleanup/handoff)
```

For each gap:
1. Identify missing capability
2. Make it legible to agent (in-repo, versioned)
3. Make it enforceable (linter, hook, structural test)
4. Have the agent write the fix

## Build a New Agent

Derive the agent from the behavior you want: seven questions (ROLE, INPUT, TOOLS, HOOKS, LOOP, MEMORY, OUTPUT) answered in order, then ratchet up from a running v0.1 as failures demand. Full recipe + building sequence: `references/building-agents.md`. Reference implementation: `.kiro/agents/autofix.json` + `.kiro/shared/autofix-system.md`.

## Is This Harness Good? (Quality Gate)

Three questions. A "no" on any means the harness has a hole.

1. **Does it move the number?** Run the task with and without the component. If completion/rework/correctness doesn't change, the component is decoration or debt — remove it.
2. **Can the agent bypass it?** If reliability depends on the agent *choosing* to comply, it's a suggestion, not a control. Good controls are mechanical (hooks, denied paths, exit gates, type system) — the agent can't opt out.
3. **Does it survive the agent?** A harness agents erode (copying bad patterns, rewriting the feature list to hide unfinished work, letting docs drift) isn't good. Anti-entropy must be built in.

To measure quality (not just gate it), read `references/measuring-harness-quality.md` — behavioral metrics (VCR, verification gap, rebuild cost, escape rate), the controlled-exclusion protocol, and the structural scoring rubric. Structural predicts; behavioral confirms.

## References (load on demand)

Navigate only when the specific topic is needed:

| Topic | File |
|-------|------|
| All theory + first principles: agent=model+harness, the ratchet, behavior-derivation, reliability-per-cost, what makes a harness good, generator/evaluator, Ashby's Law, progressive disclosure, entropy/GC, **core patterns + anti-patterns** | `references/harness-patterns.md` |
| Build a new agent: seven-question recipe + building sequence + starting templates | `references/building-agents.md` |
| This repo's maturity level + roadmap to Level 5 | `references/this-repo-status.md` |
| Session lifecycle, init checklist, handoff template, scope control, evidence | `references/session-lifecycle.md` |
| Measuring quality: behavioral metrics (VCR, verification gap, escape rate), exclusion protocol, structural rubric | `references/measuring-harness-quality.md` |
| Kiro agent JSON schema, hooks, resource URIs, multi-agent orchestration (subagent tool, DAG, review loops) | `references/kiro-agent-schema.md` |
| Built-in tool names + aliases (which name to use in `tools`/`allowedTools` vs hook `matcher`), permission defaults, per-tool `toolsSettings`, shell side channels | `references/built-in-tools.md` |
| Kiro config gotchas, field guidance, validation workflow | `references/kiro-config-guide.md` |
| Sensor commands for this repo's verify-fix loops | `references/sensor-commands.md` |

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/validate-kiro-agent.js` | Validate `.kiro/agents/*.json` against schema. Run after every config edit. |

## Deliverables Checklist

Always produce complete packages:

| Task | Must Deliver |
|------|-------------|
| Create agent config | Config JSON + system prompt + validation result |
| Add hook | Updated config + test that hook fires |
| Design verify-fix loop | System prompt section + sensor commands + exit codes |
| Refactor workflow → agent | Workflow step + agent config + system prompt |
| Improve harness subsystem | Steering/hook/structural-test + evidence it works |

## Core Patterns & Anti-Patterns

The 16 core patterns (Generator/Evaluator, Feedforward+Feedback, Keep Quality Left, WIP=1, Three-Layer Termination, Tool Gating, Structured Compaction, Progressive Disclosure, etc.) and the 9 anti-patterns live in `references/harness-patterns.md`. Load it when diagnosing a gap or applying a pattern.

## Maturity & Roadmap

Generic maturity ladder (L0 prompt-only → L5 observability + self-correction) and this repo's current level + roadmap to Level 5: `references/this-repo-status.md`.
