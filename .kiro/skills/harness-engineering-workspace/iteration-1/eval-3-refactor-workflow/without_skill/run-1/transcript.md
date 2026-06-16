# Transcript — eval-3-refactor-workflow (without_skill)

## Goal

Refactor the "Process queue" step in
`.github/workflows/kiro-autofix-trigger.yml` so Kiro is invoked through a custom
agent config (`kiro-cli --agent autofix chat --no-interactive --trust-all-tools`)
with a **minimal positional prompt**, passing the diagnostics context via the
`KIRO_DIAGNOSTICS_FILE` environment variable instead of inlining it into the
prompt. Preserve the queue loop, commit logic, truncation logic, and all safety
checks. Produce three deliverables plus this transcript. Real repo files are
read-only; deliverables go only into the `outputs/` directory.

## Steps taken

1. **Read the real workflow (read-only).** Opened
   `.github/workflows/kiro-autofix-trigger.yml` in two passes (lines 1–597 and
   597–794) to fully capture the `Process queue` step.

   Key findings about the existing step:
   - Invocation was `kiro-cli chat --agent autofix --model claude-opus-4.6
     --effort max --no-interactive --trust-all-tools "$PROMPT"`.
   - It already exported `KIRO_DIAGNOSTICS_FILE`, `KIRO_COMMIT_MSG_FILE`,
     `KIRO_GIT_HISTORY_FILE`, and `KIRO_LEARNINGS_FILE`.
   - The positional prompt was large: it inlined the per-artifact success
     criteria, instructions to read the diagnostics/history/learnings files,
     the commit-message destination, and the "no stubs" directive.
   - Safety checks present: empty-context fallback/skip, four `diag-deploy-*`
     skips (cluster unreachable, JWT_SECRET too short, PoolTimedOut, 28P01
     credential mismatch), and an "all referenced files already fixed" dedup.
   - Truncation: `MAX_CONTEXT_BYTES` guard using `tail -c` plus an appended
     truncation note.
   - Commit logic: `git add -A` → `git status --porcelain` change detection →
     commit via `-F "$KIRO_COMMIT_MSG_FILE"` (with appended `Workflow run:`
     trailer) or a three-line fallback `git commit -m ...`.
   - Exit-code handling drives the queue: non-zero agent exit stops the queue
     (after committing any partial fix); learnings + git-history files are
     updated each iteration.

2. **Read the existing agent config.** Opened `.kiro/agents/autofix.json` to
   reuse its exact schema: `tools`/`allowedTools`, `toolsSettings.write
   .deniedPaths`, `resources`, and the `agentSpawn`/`preToolUse`/`postToolUse`/
   `stop` hooks (including the hook that blocks the agent from running git, and
   the stop hook that forces verification sensors to run).

3. **Designed the refactor.**
   - Switched the invocation to the requested form:
     `kiro-cli --agent autofix chat --no-interactive --trust-all-tools "$PROMPT"`.
   - Reduced the positional prompt to a single minimal line:
     `Fix the CI failure for artifact '${artifact}' (queue position ${POS}/${TOTAL}).`
   - Moved all standing instructions into the agent system prompt
     (`autofix-system.md`).
   - Kept the diagnostics flowing through `KIRO_DIAGNOSTICS_FILE` only (never
     inlined). Moved the per-artifact success criteria out of the prompt into a
     new `KIRO_SUCCESS_CRITERIA` env var (plus `KIRO_ARTIFACT` and
     `KIRO_QUEUE_POSITION` for context), so the prompt stays minimal while
     behavior is preserved.
   - Left the loop, truncation, every safety skip, and the full commit logic
     byte-for-byte intact.

4. **Wrote the deliverables** into `outputs/`:
   - `workflow-step.yml` — the refactored `Process queue` step.
   - `autofix.json` — the agent config (system prompt referenced as
     `file://autofix-system.md`; `model` set to `claude-opus-4.6` since the
     CLI invocation no longer passes `--model`).
   - `autofix-system.md` — the agent system prompt holding the moved
     instructions, input contract, rules, workflow, commit-message format, and
     an example.

## Deliverables

- `outputs/workflow-step.yml`
- `outputs/autofix.json`
- `outputs/autofix-system.md`

## Notes / decisions

- The real workflow was treated as read-only. No file in the repo was created,
  modified, or deleted. All new files live under the `without_skill/` workspace.
- The CLI form was changed to the requested global-flag style
  (`--agent autofix` before `chat`). Because `--model`/`--effort` are dropped
  from the invocation, the model is declared in `autofix.json`.
- Success criteria moved from the prompt into `KIRO_SUCCESS_CRITERIA` so the
  positional prompt is genuinely minimal yet the per-artifact "definition of
  done" behavior is unchanged.
- The agent system prompt restates the "no git operations" boundary, matching
  the existing `preToolUse` git-blocking hook, and the "run verification
  sensors before stopping" expectation enforced by the `stop` hook.
