# Transcript — Eval 2: Verify-Fix Loop (with skill)

## Task

Design a verify-fix loop for the CI autofix agent covering a Rust workspace (backend +
WASM frontend) and a TypeScript sidecar (baileys-service). Requirements: detect modified
file types, run only relevant sensors, iterate up to a configurable max (default 3 via
`KIRO_MAX_FIX_ITERATIONS`), exit 0=clean / 1=partial / 2=no progress, order sensors
cheapest-first, put eslint as a separate TS lint sensor between build and test. Deliver as
a system prompt section.

## Steps

1. **Activated the skill.** Read `.kiro/skills/harness-engineering/SKILL.md`. The
   Deliverables Checklist row "Design verify-fix loop → System prompt section + sensor
   commands + exit codes" set the deliverable shape. The skill pointed at two references
   relevant to this task.

2. **Read `references/sensor-commands.md`.** Source of truth for exact commands:
   - Rust: `cargo fmt --all -- --check`, `cargo clippy --locked -p realestate-backend`,
     `cargo clippy --locked --target wasm32-unknown-unknown -p realestate-frontend`,
     `cargo test --locked -p realestate-backend --no-fail-fast`.
   - TS: `npm run build` (tsc) → `npx eslint . --max-warnings 0` → `npm test` (vitest).
   - Priority Order (Keep Quality Left): format → lint → compile → test, cheapest first.
   - Rationale captured: `--locked` (reproducibility), `-p` (per-crate legibility),
     `--max-warnings 0` (no eslint false negatives).

3. **Read `references/harness-patterns.md`.** Pulled the patterns that govern this design:
   - Generator/Evaluator separation (agent generates, sensors evaluate, never self-grade).
   - Keep Quality Left (cheapest-first sensor ordering, fail early).
   - Success silent / failure verbose (capture full sensor output only on fail).
   - Three-Layer Termination (static → compile → behavior; no declaring done at lint).
   - Ashby's Law / bounded variety (detect stacks, run only matching sensors).
   - Behavior-derivation (each part of the loop delivers one named behavior).

4. **Read existing harness artifacts** (`.kiro/agents/autofix.json`,
   `.kiro/shared/autofix-system.md`) to match conventions: env-var table style,
   `KIRO_MAX_FIX_ITERATIONS` default of 3, priority ordering, commit-message status field,
   and the "you own the loop" framing. The deliverable is written to slot alongside the
   existing system prompt.

## Decisions

- **Stack detection table** keyed on path globs from `structure.md` (`backend/`,
  `frontend/`, `baileys-service/`) so only activated stacks run sensors. Non-code paths map
  to "mark clean" — no wasted sensor runs.
- **Cheapest-first order** follows the reference's format → lint → compile → test, with a
  documented exception for TypeScript: `tsc`/`npm run build` runs before eslint because type
  errors make lint output noisy/spurious. This satisfies the explicit requirement that
  eslint sits as a separate sensor *between build and test*, and the rationale is stated so
  the ordering isn't read as a contradiction of Keep Quality Left.
- **`cargo fmt --all`** is shared by both Rust groups; the loop runs it once per pass to
  cover backend + frontend without duplication.
- **Exit codes** defined against an iteration-1 baseline: 0 when all sensors pass, 1 when a
  sensor flipped fail→pass or failure count strictly decreased, 2 when nothing improved.
  Progress is measured, not guessed — ties exit codes to sensor evidence (Generator/Evaluator).
- **Early stop** on the same failures across two consecutive iterations (anti-pattern #7,
  incremental patching) even if budget remains.
- **Pseudocode block** included so the loop's control flow is unambiguous to the agent.
- **Behavior Contract** added at the end (U-shaped attention — important content at the
  bottom) naming the three behaviors the section delivers.

## Output Rules Compliance

- No files in the real repo were modified, created, or deleted.
- Deliverable written only to:
  `.kiro/skills/harness-engineering-workspace/iteration-1/eval-2-verify-loop/with_skill/outputs/verify-fix-loop.md`
- This transcript written to:
  `.kiro/skills/harness-engineering-workspace/iteration-1/eval-2-verify-loop/with_skill/transcript.md`

## Deliverable Checklist (from skill)

| Required | Delivered |
|----------|-----------|
| System prompt section | yes — `verify-fix-loop.md` is a drop-in section |
| Sensor commands | yes — exact per-stack commands, cheapest-first |
| Exit codes | yes — 0/1/2 table with measured progress definition |
