# Agent Rules

## The Loop

You are a while loop, not a chatbot. Every iteration: read state, plan, act, verify, check done, loop. Never self-report "done" — prove it by running the project's build and test commands.

When context gets long, delegate to sub-agents. Each gets full cognitive budget on a focused task. A fresh context outperforms a degraded one.

## Verification

Every task ends with proof, not claims. Run the project's linter, type checker, formatter, and tests. If you cannot run verification, state what you checked and what you could not.

## Escalation

- Two failures on the same approach: stop patching. Explain the root cause, try a fundamentally different approach.
- Ambiguous requirements with 2x+ effort difference between interpretations: ask.
- Design seems flawed: raise the concern before implementing.
- Planning loop (reading files, writing TODOs, no edits after 8+ tool calls): force a write action.

## Memory

The filesystem is your memory, not the conversation. Read existing code and conventions before writing. When you discover a non-obvious fix, dependency gotcha, or framework quirk, persist it to the project's lessons file. Agent confusion is a diagnostic signal — fix the context, not just the output.

## Code

No comments. Code must be self-documenting through clear naming, small functions, and obvious structure. If code needs a comment to be understood, rename or restructure it instead. The only exception is `unsafe` blocks, which must explain why safety invariants hold.

## Delegation

Route work to specialized sub-agents in `.kiro/agents/`. Never do a job a sub-agent is built for.

| Agent | When to use | Writes files? |
|---|---|---|
| `code-planner` | Multi-file changes, new features, refactors, bug fixes needing diagnosis | `.kiro/plans/` only |
| `rust-coder` | Implementing Rust backend or frontend changes from a plan | source code |
| `kotlin-coder` | Implementing Kotlin/Android changes from a plan | source code |
| `code-reviewer` | Verifying implemented code after a coder finishes | `.kiro/plans/` only |
| `librarian` | Docs lookup, API research, version checks, "how does X work" | never |

### Plan → Code → Verify Loop

For any change touching multiple files or requiring design decisions:

1. Invoke `code-planner` with requirements → writes `.kiro/plans/{task}-plan.md`
2. Invoke `rust-coder` or `kotlin-coder` → reads plan, implements, runs fmt/clippy/tests
3. Invoke `code-reviewer` → reads plan + code, runs verification, writes `.kiro/plans/{task}-review.md`
4. If FAIL → invoke `code-planner` with review findings → revised plan → back to step 2
5. If PASS → done

Skip the loop for trivial single-file edits (typo fix, one-line change). Use judgment.

### Research Path

For "how does", "what is", "which library" questions: invoke `librarian`. Do not guess from training data when docs are available.

## Guardrails

Prefer deterministic checks over instructions. A linter catching a violation in 20ms is better than a rule saying "don't do X." Checks are model-independent and don't compete for context window attention.

When a constraint matters: encode it as a check. When it's a preference: put it in a steering file. When it's a one-time instruction: say it in chat.
