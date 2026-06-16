# Sensor Commands Reference

Per-package targeting with `--locked` for reproducible builds. Workspace-wide commands mask which package has the issue.

## Rust

```bash
cargo fmt --all -- --check
cargo clippy --locked -p realestate-backend -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown -p realestate-frontend -- -D warnings
cargo test --locked -p realestate-backend --no-fail-fast
```

Why `--locked`: exact dependency versions from `Cargo.lock`. Without it, cargo may update deps mid-build → non-determinism.

Why per-package (`-p`): agent sees exactly which crate is broken. Workspace-wide mixes errors from multiple crates.

## TypeScript (baileys-service)

```bash
cd baileys-service && npm run build          # tsc type-check
cd baileys-service && npx eslint . --max-warnings 0
cd baileys-service && npm test               # vitest
```

Why `--max-warnings 0`: without it, eslint exits 0 even with warnings → false negative.

## Priority Order (Keep Quality Left)

1. Format (`cargo fmt --check`)
2. Lint (`clippy`, `eslint`)
3. Compile (`cargo build`, `tsc`)
4. Test (`cargo test`, `npm test`)

Cheapest first. If format fails, don't waste time on tests.
