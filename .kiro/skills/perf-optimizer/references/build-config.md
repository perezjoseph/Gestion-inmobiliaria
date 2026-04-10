# Build Configuration Reference Guide

## Cargo Release Profile (`[profile.release]`)

### Default Release Settings

Cargo's built-in `release` profile prioritizes runtime speed over compile time. The defaults are reasonable but leave performance on the table for production binaries.

| Setting | Default | Optimized | Effect |
|---------|---------|-----------|--------|
| `opt-level` | `3` | `3` | Maximum runtime optimization |
| `lto` | `false` | `true` or `"thin"` | Cross-crate inlining and dead code elimination |
| `codegen-units` | `16` | `1` | Allows LLVM to optimize across the entire crate |
| `strip` | `"none"` | `true` | Removes debug symbols from the binary |
| `panic` | `"unwind"` | `"abort"` | Smaller binary, no unwinding overhead |
| `debug` | `0` | `0` | No debug info in release builds |

### Recommended Release Profile

```toml
# Cargo.toml (workspace root)
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

This configuration maximizes runtime performance and minimizes binary size at the cost of longer compile times. Suitable for production deployments where build time is not a bottleneck.

### Development-Friendly Release Profile

For CI pipelines or frequent release builds where compile time matters:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16
strip = true
```

This trades some peak performance for significantly faster compile times while still producing optimized binaries.

## Link-Time Optimization (LTO)

### What LTO Does

LTO enables LLVM to optimize across crate boundaries at link time. Without LTO, each crate is compiled and optimized independently — the linker just stitches object files together. With LTO, LLVM sees the entire program and can inline functions across crates, eliminate dead code globally, and apply whole-program optimizations.

### `lto = true` (Fat LTO) vs `lto = "thin"`

| Mode | Compile Time | Runtime Performance | Memory Usage (Compiler) |
|------|-------------|--------------------|-----------------------|
| `false` | Fastest | Baseline | Low |
| `"thin"` | Moderate increase | ~90-95% of fat LTO gains | Moderate |
| `true` (fat) | Significant increase | Maximum | High |

### When to Use Each

Fat LTO (`lto = true`) merges all codegen units into a single LLVM module before optimization. This gives LLVM maximum visibility for inlining and dead code elimination but requires substantially more memory and time.

```toml
# Production binary — maximum optimization, compile time is acceptable
[profile.release]
lto = true
```

Thin LTO (`lto = "thin"`) runs optimizations in parallel on smaller modules while still enabling cross-module inlining. It captures most of the performance benefit with much less compile-time overhead.

```toml
# CI builds — good optimization with reasonable compile times
[profile.release]
lto = "thin"
```

### LTO Impact on This Project

This workspace has two crates (`realestate-backend` and `frontend`) plus many dependencies (`actix-web`, `sea-orm`, `tokio`, `serde`). LTO is especially valuable here because:

- `actix-web` handler dispatch involves trait objects and generic functions across crate boundaries — LTO enables inlining these hot paths
- `sea-orm` query builders use heavy generics that benefit from cross-crate monomorphization
- `serde` serialization/deserialization generates code in each crate that LTO can deduplicate
- `tokio` runtime internals can be inlined into application async functions

### LTO and `cdylib` / `proc-macro` Crates

LTO does not apply to `cdylib` or `proc-macro` crate types. For this project's `frontend` crate (compiled to WASM via Trunk), LTO still applies because `wasm32` targets use `lto = true` effectively — `wasm-opt` performs additional optimization passes after LLVM.

## Codegen Units (`codegen-units`)

### How Codegen Units Work

Cargo splits each crate into multiple codegen units (default: 16 in release) for parallel compilation. Each unit is optimized independently by LLVM. More units means faster compilation but less optimization — LLVM cannot inline or eliminate dead code across unit boundaries.

### `codegen-units = 1`

Setting `codegen-units = 1` forces LLVM to process the entire crate as a single unit. This enables maximum intra-crate optimization at the cost of serialized compilation.

```toml
[profile.release]
codegen-units = 1
```

### When It Matters

The benefit of `codegen-units = 1` is most noticeable when:

- Functions call each other frequently within the same crate (hot internal paths)
- The crate has many small utility functions that benefit from inlining
- Combined with LTO, it gives LLVM the broadest possible optimization scope

For this project's backend crate, the handler → service → entity call chains within `realestate-backend` benefit from single-unit compilation because LLVM can inline service functions directly into handlers.

### Trade-Off

| `codegen-units` | Compile Time | Optimization Quality |
|-----------------|-------------|---------------------|
| `16` (default) | Faster (parallel) | Good |
| `1` | Slower (serial) | Maximum |

When combined with `lto = true`, setting `codegen-units = 1` has diminishing returns because LTO already merges units at link time. The combination still produces the best results but the marginal gain over `lto = true` alone is smaller.

## `opt-level` Selection

### Available Levels

| Level | Meaning | Use Case |
|-------|---------|----------|
| `0` | No optimization | Debug builds (default for `dev`) |
| `1` | Basic optimizations | Faster debug builds with some optimization |
| `2` | Most optimizations | Balance of speed and compile time |
| `3` | All optimizations including vectorization | Release builds (default for `release`) |
| `"s"` | Optimize for binary size | Embedded or WASM targets |
| `"z"` | Aggressively optimize for size | Minimal binary size, may sacrifice speed |

### `opt-level = 3` for Backend

The backend is a long-running server process where runtime performance matters more than binary size. `opt-level = 3` enables loop vectorization, aggressive inlining, and other optimizations that benefit request throughput.

```toml
[profile.release]
opt-level = 3
```

### `opt-level = "s"` or `"z"` for WASM Frontend

For the frontend crate compiled to WASM, binary size directly affects page load time. Consider optimizing for size:

```toml
# Per-crate override for the frontend WASM binary
[profile.release.package.realestate-frontend]
opt-level = "s"
```

This reduces the `.wasm` file size while maintaining reasonable runtime performance. Use `"z"` only if size is critical and you've benchmarked the performance impact.

### Per-Dependency Optimization

Override `opt-level` for specific dependencies without affecting your own code:

```toml
# Optimize heavy dependencies even in dev builds for faster iteration
[profile.dev.package.sea-orm]
opt-level = 2

[profile.dev.package.sqlx]
opt-level = 2
```

This speeds up database operations during development without slowing down recompilation of your own code.

## `strip = true` for Binary Size

### What Stripping Does

`strip = true` removes symbol tables and debug information from the final binary. This has no effect on runtime performance but significantly reduces binary size.

| `strip` Value | Effect | Binary Size Impact |
|---------------|--------|-------------------|
| `"none"` | Keep all symbols | Baseline |
| `"debuginfo"` | Remove debug info, keep symbol names | ~30-50% smaller |
| `true` / `"symbols"` | Remove all symbols and debug info | ~50-70% smaller |

### Recommended Setting

```toml
[profile.release]
strip = true
```

For production deployments, stripping is almost always desirable. The only reason to keep symbols is for profiling or debugging production crashes with tools like `perf` or `gdb`.

### When to Keep Symbols

If you need to profile the production binary or capture meaningful backtraces:

```toml
# Profiling build — keep symbols for perf/flamegraph
[profile.release]
strip = false
debug = 1  # line tables only, minimal size increase
```

### Impact on This Project

The `realestate-backend` binary includes `actix-web`, `sea-orm`, `tokio`, and many other dependencies. Without stripping, the release binary can be 50-100MB+. With `strip = true`, expect 15-30MB — a significant reduction for container images and deployment artifacts.

## `panic = "abort"` vs `"unwind"`

### Trade-Off

| Mode | Binary Size | Runtime Overhead | Catch Panics? |
|------|------------|-----------------|---------------|
| `"unwind"` (default) | Larger (unwind tables) | Small overhead per function | Yes (`catch_unwind`) |
| `"abort"` | Smaller | None | No |

### When to Use `"abort"`

For server applications like this backend, `panic = "abort"` is appropriate when:

- You don't use `std::panic::catch_unwind` anywhere
- Panics indicate unrecoverable bugs (the process should restart)
- You want smaller binaries and slightly faster code

```toml
[profile.release]
panic = "abort"
```

### Caveat with Actix-web

Actix-web catches panics in handlers by default to prevent a single bad request from crashing the server. With `panic = "abort"`, a panic in any handler terminates the entire process. This is acceptable if your error handling is thorough (using `Result<T, AppError>` everywhere) and panics only occur on genuine bugs.

## Complete Recommended Profiles

### Production Backend

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Maximum performance, minimum binary size. Compile time: 2-5× longer than default release.

### CI / Staging

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16
strip = true
```

Good performance with faster compile times. Suitable for CI pipelines where build minutes cost money.

### Development with Fast Dependencies

```toml
[profile.dev]
opt-level = 0

[profile.dev.package."*"]
opt-level = 2
```

Your code compiles fast with no optimization. Dependencies (which change rarely) are optimized so database queries and serialization aren't painfully slow during development.

## Measuring the Impact

### Binary Size

```bash
# Compare binary sizes across profiles
cargo build --release 2>&1 && ls -lh target/release/realestate-backend

# With different LTO settings, compare:
# 1. Default (no LTO): baseline
# 2. lto = "thin": moderate reduction
# 3. lto = true + codegen-units = 1 + strip = true: maximum reduction
```

### Runtime Performance

```bash
# Use cargo's built-in benchmarking or hyperfine for CLI tools
cargo bench --release

# For the web server, use wrk or oha for HTTP benchmarking
oha -z 30s -c 50 http://localhost:8080/api/propiedades
```

### Compile Time

```bash
# Time a clean release build
cargo clean && time cargo build --release

# Compare with different profiles to find your sweet spot
```

Always benchmark before and after profile changes. The impact varies significantly based on codebase size, dependency tree, and target architecture.
