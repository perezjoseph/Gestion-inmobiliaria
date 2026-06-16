# Build a New Agent (Construction Recipe)

Derive the agent from the behavior you want. Answer seven questions in order. Every component must name the behavior it delivers — if you can't name it, delete it.

| # | Question | Produces |
|---|----------|----------|
| 1 | ROLE — one job. What it does AND what it must NOT do | `prompt` constraints |
| 2 | INPUT — structured context in (env vars, files), not vague prompt | env var table in prompt |
| 3 | TOOLS — minimum viable set + denied paths. Prefer few atomic-outcome tools over many granular ones (combine to shrink error surface) | `tools` + `toolsSettings.write.deniedPaths` |
| 4 | HOOKS — what to enforce mechanically | `preToolUse` block, `postToolUse` format, `stop` gate |
| 5 | LOOP — sensors + iteration budget + escalation rule | verify-fix section in prompt |
| 6 | MEMORY — how it avoids repeating failures | history/learnings file in prompt |
| 7 | OUTPUT — structured artifact + exit codes | output contract in prompt |

Reference implementation: `.kiro/agents/autofix.json` + `.kiro/shared/autofix-system.md` implement all seven. Study it before building a new one.

## Starting Templates

Copy these, fill the `REPLACE` slots, delete `_comment`/`_why` annotations:

| Template | Produces |
|----------|----------|
| `assets/agent-template.json` | config skeleton — the 7-pattern slots, schema-correct (file:// resolution, internal `matcher` names, deniedPaths, stop-gate) |
| `assets/system-prompt-template.md` | companion prompt — role/constraints/env-vars/loop/sensors/output-contract |
| `assets/trace-hooks.json` | standard OTel trace-emitter hook set (dual-sink → Tempo + eval file); merge into the config's `hooks` |

The agent template's stop-hook already implements the "files modified but no sensor ran → block" gate — the mechanical control that makes premature completion impossible. The prompt template's sections map 1:1 to the recipe patterns. autofix is the filled-in example.

## Building Sequence (don't design all seven upfront)

You can't ratchet without a running v0.1, and you can't get a v0.1 by perfecting the harness on paper. Optimize for TTFF (time to first feedback):

1. **v0.1** — ROLE + INPUT + minimal TOOLS, single agent thread. Get it running and producing output. That output is your learning signal.
2. **Observe failures.** Each failure names the next subsystem to add (verification gap → add sensors + stop-gate; scope creep → tighten denied paths; repeats failures → add memory).
3. **Ratchet up.** Add HOOKS, LOOP, MEMORY, OUTPUT only as failures demand them. Every addition traces to a real failure.
4. **Subagents last.** Stay single-threaded until you have a specific need for specialization or parallelization.

The prompt is your primary feedforward lever — the highest behavior-shaping per unit effort. Spend time here first. Mechanical controls (hooks, gates) come second, for the invariants you can't allow to fail. Both required (feedforward + feedback); the prompt is where v0.1 leverage lives.
