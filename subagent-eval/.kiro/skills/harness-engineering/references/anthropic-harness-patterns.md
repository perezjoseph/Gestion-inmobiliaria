# Anthropic Harness Patterns

Distilled from "Harness design for long-running application development" (Anthropic Engineering, March 2026).

## Generator/Evaluator Architecture

A multi-agent structure inspired by GANs: one agent generates, another evaluates.

**Why it works:** Agents reliably skew positive when grading their own work. Separating generation from evaluation creates a feedback loop that drives quality upward. Tuning a standalone evaluator to be skeptical is far more tractable than making a generator critical of its own output.

**Implementation:**
- Generator agent: reads diagnostics, produces code fixes
- Evaluator: computational sensors (fmt, clippy, tests) that grade output objectively
- The evaluator's findings flow back as input for the next generator iteration
- Run 1–5 iterations per fix, with each iteration responding to evaluator feedback

## Context Resets vs Compaction

**Problem:** Models lose coherence on lengthy tasks as context fills. Some exhibit "context anxiety" — wrapping up prematurely near perceived context limits.

**Context reset:** Clear the context window entirely, start a fresh agent with structured handoff carrying previous state and next steps.

**Compaction:** Summarize earlier conversation in place so the same agent continues on shortened history.

**When to use resets:** Multi-artifact queues, long-running tasks, when the model exhibits premature wrap-up behavior. Each artifact gets a clean slate.

**Handoff artifacts must include:**
- What was accomplished so far
- What still needs to be done
- Relevant state (file paths modified, errors encountered, iteration count)

## Sprint Contracts

Before building, the generator and evaluator agree on what "done" looks like:
- Generator proposes what it will build and how success is verified
- Evaluator reviews the proposal to ensure correctness criteria are testable
- Both iterate until they agree on the contract

In CI autofix context: the "contract" is the set of sensors that must pass (fmt clean, clippy clean, tests green).

## Evaluator Tuning

Out of the box, LLMs are poor QA agents:
- They identify issues then talk themselves into deciding they're not a big deal
- They test superficially rather than probing edge cases
- They approve work that a human would reject

**Tuning approach:** Read evaluator logs, find where judgment diverges from yours, update the evaluator's prompt to address those gaps. Several rounds of this loop are needed.

For computational evaluators (linters, tests), this problem doesn't exist — they're deterministic and objective.

## Simplify as Models Improve

Every harness component encodes an assumption about what the model can't do alone. These assumptions go stale as models improve.

**Practice:** When a new model lands, re-examine the harness. Strip pieces that are no longer load-bearing. Add new pieces to achieve greater capability.

The space of interesting harness combinations doesn't shrink — it moves.
