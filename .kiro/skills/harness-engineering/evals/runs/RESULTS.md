# Eval Run Results — harness-engineering

Skill version 3.0. Each eval prompt was executed fresh applying the skill; deliverables written under `runs/eval-N/`, then graded against `evals.json` expectations.

## Scorecard

| Eval | Topic | Score | Verdict |
|------|-------|-------|---------|
| 1 | Create kiro-cli agent config | 8/8 | PASS |
| 2 | Design verify-fix loop | 8/8 | PASS |
| 3 | Refactor workflow → agent (3-file package) | 9/9 | PASS |
| **Total** | | **25/25** | **PASS** |

## Eval 1 — agent config + system prompt

Files: `eval-1/agents/autofix.json`, `eval-1/prompts/autofix-system.md`

| # | Expectation | Result |
|---|-------------|--------|
| 1 | Valid JSON at autofix.json | PASS |
| 2 | `prompt` file:// URI resolves relative to config dir (`../prompts/...`) | PASS |
| 3 | `resources` use glob (`**/*.md`) for steering, not individual files | PASS |
| 4 | `hooks` uses `postToolUse` trigger key | PASS |
| 5 | Hook uses `matcher` field with internal name `fs_write` | PASS |
| 6 | `allowedTools` present, no `*` wildcard | PASS |
| 7 | Passes validate-kiro-agent.js (exit 0) | PASS |
| 8 | Companion system prompt produced (complete package) | PASS |

Validator: `valid: true, errors: []`.

## Eval 2 — verify-fix loop

File: `eval-2/verify-fix-loop.md`

| # | Expectation | Result |
|---|-------------|--------|
| 1 | Sensors ordered cheapest-first (format → lint → compile → test) | PASS |
| 2 | Selective activation (Rust for `.rs`, TS for baileys-service) | PASS |
| 3 | Correct Rust sensor commands (fmt check, clippy backend, clippy wasm frontend) | PASS |
| 4 | eslint as distinct sensor (`npx eslint . --max-warnings 0`) | PASS |
| 5 | `npm run build` and `npm test` as separate TS sensors | PASS |
| 6 | Three exit codes (0=clean, 1=partial, 2=no progress) | PASS |
| 7 | Different approach after 2 identical failures (not incremental) | PASS |
| 8 | Iteration cap via `KIRO_MAX_FIX_ITERATIONS`, default 3 | PASS |

## Eval 3 — workflow refactor package

Files: `eval-3/workflow-step.yml`, `eval-3/agents/autofix.json`, `eval-3/prompts/autofix-system.md`

| # | Expectation | Result |
|---|-------------|--------|
| 1 | Uses `kiro-cli --agent autofix chat --no-interactive --trust-all-tools` | PASS |
| 2 | Diagnostics via `KIRO_DIAGNOSTICS_FILE` (not inlined) | PASS |
| 3 | Minimal positional prompt (artifact name + queue position) | PASS |
| 4 | Commit logic preserved (git add -A, status check, commit -F / fallback) | PASS |
| 5 | `KIRO_COMMIT_MSG_FILE` still set per artifact | PASS |
| 6 | Queue loop (`while read artifact`) preserved | PASS |
| 7 | Truncation logic for oversized diagnostics preserved | PASS |
| 8 | Companion agent config with `matcher: fs_write` hook | PASS |
| 9 | Companion system prompt documents all env vars | PASS |

Validator: `valid: true, errors: []`.

## Notes

- Both generated agent configs pass the bundled validator with zero errors.
- All three runtime gotchas the skill warns about (hook `matcher`/`fs_write`, file:// relative resolution, `allowedTools` no-`*`) were handled correctly without prompting — evidence the progressive-disclosure references carry the load.
- No expectation required behavior the skill failed to deliver. Skill is healthy at v3.0.
