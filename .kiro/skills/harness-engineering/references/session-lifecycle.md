# Session Lifecycle

Operational patterns for structured agent sessions, initialization, handoff, and multi-session continuity.

## The Structured Session

```
START
  1. Read instructions (AGENTS.md / steering)
  2. Run init (install, verify, health check)
  3. Read progress state (what happened last time)
  4. Read task state (what's done, what's next)

SELECT
  5. Pick exactly ONE unfinished task
  6. Work only on that task

EXECUTE
  7. Implement
  8. Run verification sensors
  9. Fail → fix and re-run
  10. Pass → record evidence

WRAP UP
  11. Update progress/task state
  12. Record what's broken or unverified
  13. Commit only when safe to resume
  14. Leave clean restart path
```

## Kiro Mapping

| Phase | Mechanism |
|-------|-----------|
| Init/health | `preTaskExecution` hook |
| Progress read | Spec states, git log |
| Feature select | Spec task `in_progress` |
| Verification | `postToolUse` hooks, `postTaskExecution` hooks |
| Wrap up | `agentStop` hook, git-commit-push hook |

## Init Checklist

Before ANY work:

- [ ] Dependencies installed (no stale lockfile)
- [ ] Build compiles clean
- [ ] Lint passes
- [ ] Tests pass
- [ ] Working directory clean

Fail → fix FIRST or document as known issue. Never start on broken foundation.

## Session Handoff Template

```markdown
## Verified Now
- Working: [what passes]
- Ran: [exact commands + exit codes]

## Changed This Session
- Code: [files added/modified]
- Harness: [steering/hooks/infra changes]

## Broken Or Unverified
- Defect: [known bugs]
- Unverified: [paths not tested]
- Risk: [what could break next session]

## Next Step
- Task: [highest priority unfinished]
- Passing criteria: [what counts as done]
- Don't touch: [what must not change]
```

## Scope Control

- One task at a time. No overreach.
- Don't modify task list to hide unfinished work.
- Don't start new task until current passes verification.
- "Done" = sensor evidence, not self-assessment.

## Evidence-Based Completion

```
NOT evidence: "should work" / "looks correct"
IS evidence:  "cargo test passed (exit 0)" / "curl returned 200"
```

## Multi-Session State

| What | Storage | When Read |
|------|---------|-----------|
| Done | Spec tasks (completed) | Session start |
| In progress | Spec tasks (in_progress) | Session start |
| What broke | Git log, test output | Session start |
| Decisions | Steering files | On demand |
| Architecture | Code + types | On demand |
