# Transcript — Refactor "Process queue" → custom agent invocation

Task: refactor the `Process queue` step in `.github/workflows/kiro-autofix-trigger.yml`
to invoke a custom agent config via the headless form
`kiro-cli --agent autofix chat --no-interactive --trust-all-tools` with a **minimal
positional prompt**, passing diagnostics context through `KIRO_DIAGNOSTICS_FILE`
instead of inlining it. Keep the queue loop, commit logic, truncation logic, and all
safety checks intact. Real repo files are READ-ONLY; all output goes to the eval
output directory.

## Steps

1. **Activated the skill.** Read `harness-engineering/SKILL.md`. This is a
   "Refactor workflow → agent" task, so the Deliverables Checklist requires a complete
   package: **workflow step + agent config + system prompt**. Loaded the two references
   the skill points at for this work: `references/kiro-agent-schema.md` (headless mode,
   schema) and `references/kiro-config-guide.md` (gotchas + validation workflow). Also
   read `references/sensor-commands.md` for the verify-fix sensor commands.

2. **Read the real workflow (read-only).** Studied the whole `Process queue` step plus
   the steps it interacts with (history, commit-message trailer, push). Captured the
   pieces that must be preserved verbatim:
   - Queue loop `while IFS= read -r artifact ... done < <(jq -r '.[]')`.
   - Empty/alternative-file fallback and the `MAX_CONTEXT_BYTES` truncation block.
   - All five safety skips: cluster unreachable, `JWT_SECRET` too short, `PoolTimedOut`,
     password auth (28P01), and the already-fixed-files dedup.
   - Commit logic: `git add -A`, `git status --porcelain` check, commit via
     `KIRO_COMMIT_MSG_FILE` (with the `Workflow run:` trailer appended) or the
     `fix(autofix): resolve <artifact>` fallback; learnings + git-history refresh;
     exit-code handling (0 → continue, non-zero → commit partial + break).
   - `GH_TOKEN` is deliberately NOT exported to the agent.

3. **Read the existing agent + system prompt** (`.kiro/agents/autofix.json`,
   `.kiro/shared/autofix-system.md`) to refactor consistently rather than invent a new
   shape.

## Key decisions

- **Minimal prompt, context via env vars.** The original built a long positional prompt
  that inlined the success criteria, artifact name, queue position, and file
  references. The refactor reduces the prompt to:
  `"Fix the CI failure described in the diagnostics. Read $KIRO_DIAGNOSTICS_FILE and follow your system prompt."`
  Everything else moves to environment variables.

- **New env vars** exported per artifact, alongside the originals
  (`KIRO_DIAGNOSTICS_FILE`, `KIRO_COMMIT_MSG_FILE`, `KIRO_GIT_HISTORY_FILE`,
  `KIRO_LEARNINGS_FILE`): `KIRO_ARTIFACT_NAME`, `KIRO_QUEUE_POSITION`,
  `KIRO_SUCCESS_CRITERIA`. The per-artifact `case` that computes the success criteria is
  kept exactly as-is; its result is now exported instead of string-concatenated into the
  prompt. This directly addresses config-guide Gotcha #4 (env vars passed by the workflow
  MUST be documented in the system prompt) — the system prompt now leads with an
  env-var table.

- **Invocation form** changed to the documented headless order from
  `kiro-agent-schema.md`: `--agent` before the `chat` subcommand, then
  `--no-interactive --trust-all-tools`. Per the task's explicit command form, the
  `--model` / `--effort` flags were dropped from this invocation; model selection now
  lives in the agent config (`"model": "claude-opus-4.6"`), keeping the call site
  minimal and the model choice in one place.

- **Agent config** is based on the existing real config (same denied write paths,
  git-blocking `preToolUse` hook, fmt/eslint/spotless `postToolUse` hooks, and the
  sensor-gate `stop` hook that blocks "done" until sensors ran — the Verification
  subsystem). `prompt` stays `file://../shared/autofix-system.md` so it resolves to
  `.kiro/shared/autofix-system.md` when the config is deployed at `.kiro/agents/`
  (config-guide Gotcha #2). `model` set so the call site needs no `--model`.

- **Tools simplified for validity.** The validator rejected `code`, `introspect`, and
  `thinking` as tool references. Replaced the explicit built-in list with `@builtin`
  (covers all built-in tools) plus `@context7`, and listed the auto-approved subset in
  `allowedTools` (`@builtin` covers capability; `allowedTools` lists specific patterns
  since it does not accept `*` — Gotcha #3).

## Validation

Ran the skill's validator:

```
node .kiro/skills/harness-engineering/scripts/validate-kiro-agent.js \
  .../with_skill/outputs/autofix.json
```

After the tools fix, two messages remain, both expected and not real defects:

1. `prompt: file not found` — the validator resolves `file://../shared/...` relative to
   the JSON's location. The deliverables live in a flat `outputs/` dir, so it looks for
   `outputs/../shared/autofix-system.md`. When deployed at `.kiro/agents/autofix.json`
   the path resolves to `.kiro/shared/autofix-system.md`, which exists. The path is
   correct for deployment (Gotcha #2).
2. `mcpServers.context7.command: expected non-empty string` — the validator assumes a
   stdio server with a `command`. `context7` is an **HTTP** MCP server (`type` + `url`),
   which is the form documented in `kiro-agent-schema.md` and used by the existing real
   config. No `command` applies. This is a validator gap, not a config error.

The config parses as valid JSON (`ConvertFrom-Json` OK). Manual checks of the
non-validator-catchable gotchas (#1 hook matcher = internal names `fs_write`/`str_replace`/
`execute_bash`; #4 env vars documented in the system prompt) pass.

## Deliverables

- `outputs/workflow-step.yml` — refactored `Process queue` step.
- `outputs/autofix.json` — agent config (model set, `@builtin` tools, write-path denials,
  git-block + fmt + sensor-gate hooks).
- `outputs/autofix-system.md` — system prompt; leads with the env-var contract
  (`KIRO_DIAGNOSTICS_FILE`, `KIRO_ARTIFACT_NAME`, `KIRO_SUCCESS_CRITERIA`,
  `KIRO_QUEUE_POSITION`, `KIRO_GIT_HISTORY_FILE`, `KIRO_LEARNINGS_FILE`,
  `KIRO_COMMIT_MSG_FILE`, `KIRO_MAX_FIX_ITERATIONS`).

No real-repo files were modified.
