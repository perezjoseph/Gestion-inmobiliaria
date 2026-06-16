# Verify-Fix Loop

You fix CI failures across a Rust workspace (`realestate-backend` + WASM `realestate-frontend`) and a TypeScript sidecar (`baileys-service`). After every fix you run sensors to prove the fix. Passing sensors are the only valid evidence.

<selective_activation>
Run only the sensors relevant to the files you modified. Do not run Rust sensors for a TypeScript-only change, or TypeScript sensors for a Rust-only change.

- Modified any `*.rs` file or `Cargo.toml`/`Cargo.lock` → run the Rust sensors.
- Modified any file under `baileys-service/` → run the TypeScript sensors.
- Modified both → run both sensor groups.
</selective_activation>

<sensors>
Sensors are ordered cheapest-first. Run them in order and stop at the first failure — do not run tests if format fails.

## Rust sensors (when `*.rs` changed)

1. Format: `cargo fmt --all -- --check`
2. Lint (backend): `cargo clippy --locked -p realestate-backend -- -D warnings`
3. Lint (frontend): `cargo clippy --locked --target wasm32-unknown-unknown -p realestate-frontend -- -D warnings`
4. Test: `cargo test --locked -p realestate-backend --no-fail-fast`

## TypeScript sensors (when `baileys-service/` changed)

1. Lint: `cd baileys-service && npx eslint . --max-warnings 0`
2. Compile: `cd baileys-service && npm run build`
3. Test: `cd baileys-service && npm test`

Cost order across the whole loop: format → lint → compile/typecheck → test.
</sensors>

<iteration>
Iterate fix → sensors → fix until sensors pass or the cap is hit.

- The iteration cap is read from `KIRO_MAX_FIX_ITERATIONS` (default: 3 when unset).
- After two consecutive identical sensor failures, do NOT patch incrementally. Diagnose the root cause and try a fundamentally different approach.
</iteration>

<exit_codes>
Exit with the code that matches the outcome:

| Code | Meaning |
|------|---------|
| 0 | Clean — all relevant sensors pass |
| 1 | Partial — some sensors fixed, others still failing after the cap |
| 2 | No progress — sensor state is unchanged from the start |
</exit_codes>
