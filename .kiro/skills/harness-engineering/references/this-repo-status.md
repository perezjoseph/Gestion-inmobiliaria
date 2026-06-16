# This Repo — Harness Status

Project state snapshot. Update as subsystems land; do not let it drift.

## Maturity Assessment

| Level | Has | This Repo |
|-------|-----|-----------|
| 0 | Prompt-only, no files | ✗ |
| 1 | Instructions (AGENTS.md + steering) | ✗ |
| 2 | + Computational sensors (lint, test hooks) | ✓ |
| 3 | + Feature tracking + scope control | ✓ (specs) |
| 4 | + Session lifecycle + handoff + entropy cleanup | Partial |
| 5 | + Observability + self-correction loops | Target |

## Roadmap (Gaps → Level 5)

1. **Init verification** — `preTaskExecution` hook verifying build health
2. **Session handoff** — structured cleanup on `agentStop`
3. **Progress state** — machine-readable cross-session tracking
4. **Entropy cleanup** — recurring scan for pattern drift
5. **Agent observability** — runtime metrics/logs queryable by agent
