---
name: verify-fix-loop
description: >
  Activate after applying any code fix, when running verification sensors, or
  when iterating on failed checks. Encodes the diagnose → fix → verify → repeat
  workflow with priority ordering and exit conditions. Covers Rust (cargo clippy,
  cargo test, cargo fmt), TypeScript (tsc, ESLint, npm test in baileys-service),
  Kotlin (gradle build), Python (ruff, pytest), and non-code validators
  (hadolint, kubeconform, shellcheck) verification loops.
---

# Verify-Fix Loop

Iterative workflow for applying code fixes and verifying correctness through deterministic feedback sensors. The agent owns the loop internally — the workflow does not re-invoke.

## Loop Workflow

Execute these steps in order on every iteration:

1. **Detect modified file types** — Run `git diff --name-only` against the pre-fix state. Classify each file by stack.
2. **Select applicable sensors** — Activate only sensors relevant to the modified file types (see Sensor Selection below).
3. **Run all selected sensors** — Execute each sensor, capturing stdout, stderr, and exit code.
4. **Parse failures into actionable findings** — Extract file:line:column, error codes, lint IDs, and test names from sensor output.
5. **Apply fixes for highest-priority findings first** — Address failures in priority order (see Priority Ordering below).
6. **Re-run sensors** — Verify the fix resolved the issue without introducing new failures.
7. **Repeat** — Continue until all sensors pass or max iterations reached.

## Sensor Selection by File Type

| Modified files | Sensors to run |
|----------------|----------------|
| `*.rs` | Rust: `cargo fmt --all -- --check` → `cargo clippy --locked -p realestate-backend -- -D warnings` → `cargo clippy --locked --target wasm32-unknown-unknown -p realestate-frontend -- -D warnings` → `cargo test --locked -p realestate-backend --no-fail-fast` |
| `baileys-service/**/*.{ts,tsx,json}` | TypeScript: `cd baileys-service && npm run build` → `cd baileys-service && npx eslint . --max-warnings 0` → `cd baileys-service && npm test` |
| `android/**/*.{kt,kts}` | Kotlin: `cd android && ./gradlew build` (scoped to affected module) |
| `ocr-service/**/*.py` | Python: `cd ocr-service && ruff format --check .` → `cd ocr-service && ruff check .` → `cd ocr-service && python -m pytest -q` |
| `**/Dockerfile`, `*.Dockerfile` | Docker: `hadolint <path>` |
| `infra/k8s/**/*.{yml,yaml}` | Kubernetes: `kubeconform -strict -ignore-missing-schemas <path>` |
| `*.sh` | Shell: `shellcheck <path>` |
| Mixed stacks | All applicable sensor suites |
| Non-code only (YAML outside k8s, Markdown, config) | None — mark verification as clean |

## Priority Ordering

When multiple sensors fail simultaneously, fix in this order:

1. **Compilation errors** (highest priority) — Type errors, missing imports, syntax errors. Nothing else can pass if the code doesn't compile.
2. **Lint warnings** — Clippy warnings (`-D warnings` makes them errors), ESLint violations. Fix these before running tests since lint issues often indicate logic bugs.
3. **Test failures** (lowest priority) — Unit and integration test failures. Address only after compilation and lint are clean.

Within the same priority level, fix errors in dependency order: imports before usages, type definitions before implementations, upstream modules before downstream consumers.

## Iteration Strategy

- **Default**: Apply the minimal, most targeted fix for each finding. Prefer compiler-suggested fixes and LSP code actions over manual edits.
- **After 2 consecutive iterations with the same error**: The current approach is fundamentally wrong. Do NOT make incremental patches. Step back and try a completely different approach:
  - If a type error persists, reconsider the type design rather than adding casts.
  - If a test keeps failing, re-read the test to understand what it actually expects rather than tweaking the implementation.
  - If a lint warning recurs, address the underlying pattern rather than suppressing or working around it.
- **Never suppress warnings**: Do not add `#[allow(...)]`, `// @ts-ignore`, `// eslint-disable`, or similar suppressions. Fix the root cause.

## Exit Conditions

| Exit Code | Condition | Meaning |
|-----------|-----------|---------|
| **0** | All selected sensors pass | Clean fix — commit and continue |
| **1** | Max iterations reached, but progress was made (fewer errors than initial state) | Partial fix — commit what works, report remaining failures |
| **2** | Max iterations reached with no progress (same errors persist across iterations) | No fix possible with current approach — do not commit, escalate |

### Determining Progress

Progress is measured by comparing the set of errors between iterations:
- **Progress made**: The number of distinct errors decreased, OR at least one error was resolved (even if new ones appeared).
- **No progress**: The exact same errors (same file, same line, same error code) persist across two consecutive iterations with no reduction.

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `KIRO_MAX_FIX_ITERATIONS` | Maximum loop iterations before forced exit | 3 |
| `KIRO_DIAGNOSTICS_FILE` | Path to initial diagnostics context | (required) |
| `KIRO_COMMIT_MSG_FILE` | Path where commit message is written | (required) |

## References

- **`references/typescript-fixes.md`** — Load when fixing TypeScript compilation errors, ESLint violations, or test failures in `baileys-service/`. Contains dependency-order resolution strategy, auto-fix workflow, and suppression comment prohibition.

## Commit Message on Exit

On exit 0 or 1, write a structured commit message to `KIRO_COMMIT_MSG_FILE`:

```
fix(<scope>): <subject under 70 chars>

Root cause:
- <error identifiers: clippy lint IDs, test names, file:line, TS error codes>

Changes:
- <bullet per modified file or logical change>

Verification:
- <sensor name>: PASS|FAIL
- <sensor name>: PASS|FAIL

Status: CLEAN|PARTIAL
Iteration: <n>/<max>
Workflow run: <url>
```

On exit 2, do not commit. The workflow handles escalation.
