# Transcript — Verify-Fix Loop Design (without skill)

## Task
Design a verify-fix loop system prompt section for the CI autofix agent covering a
Rust workspace (backend + WASM frontend) and a TypeScript sidecar (baileys-service).
Loop must: detect modified file types, run only relevant sensors, iterate up to a
configurable max (default 3, `KIRO_MAX_FIX_ITERATIONS`), exit 0/1/2, order sensors
cheapest-first, and include eslint as a separate TS lint sensor between build and test.

## Constraints honored
- No real-repo files modified, created, or deleted. Deliverables written only to the
  designated output directory and the eval root.
- Did not read any `SKILL.md` file.

## Steps taken
1. Read existing harness config and references to ground the sensor commands in what
   the repo actually uses:
   - `.kiro/agents/autofix.json` — confirmed deniedPaths, stop-gate behavior, the
     `cargo fmt / cargo clippy / cargo test / npm run build / npm test / npx eslint`
     sensor vocabulary already tracked by hooks, and the `$RUNNER_TEMP/autofix-modified-files.txt`
     side-channel the loop reads to detect modified stacks.
   - `.kiro/skills/harness-engineering/references/sensor-commands.md` — exact per-package
     commands, `--locked` and `-p` rationale, `--max-warnings 0` rationale, and the
     existing "Keep Quality Left" priority order (format < lint < compile < test).
   - `.kiro/specs/autofix-harness-expansion/design.md` — confirmed stack model
     (Rust / TS / Kotlin / Python), per-stack sensor-ran tracking, and exit-status
     vocabulary used elsewhere in the harness.
2. Confirmed package identities so commands are correct, not guessed:
   - `backend/Cargo.toml` -> crate `realestate-backend`.
   - `sensor-commands.md` -> frontend crate `realestate-frontend`, target
     `wasm32-unknown-unknown`.
   - `baileys-service/package.json` -> scripts `build` = `tsc`, `test` = `vitest run`;
     eslint invoked via `npx eslint . --max-warnings 0` (eslint is a standalone sensor,
     not an npm script), placed between build and test per the task.
3. Wrote `verify-fix-loop.md` as a system-prompt section structured for LLM compliance
   (front-loaded `<rules>`, one constraint per bullet, positive directives, a detection
   table, literal command blocks, bounded-iteration pseudocode, an exit-code table, and
   a worked input/output example).

## Design decisions
- Detection keys off the existing modified-files side-channel and maps each path to
  exactly one stack; only touched stacks get sensors (skips frontend when untouched).
- Cheapest-first ordering enforced per stack: Rust = fmt -> clippy -> test;
  TS = build (tsc) -> eslint -> test (vitest). WASM frontend uses clippy as its
  compile sensor and runs no `cargo test` (will not build natively).
- Iteration bounded by `KIRO_MAX_FIX_ITERATIONS` (default 3) with a no-net-progress
  early stop so the agent does not burn budget while stuck.
- Exit codes: 0 = all sensors pass; 1 = partial (failures remain but progress made);
  2 = no progress / untouched failure after max iterations.
- Kept the no-suppression rule (rule 5) so the agent cannot fake green — consistent
  with the harness's existing suppression stance.

## Deliverables produced
- `outputs/verify-fix-loop.md` — the system prompt section.
- `transcript.md` — this file.
