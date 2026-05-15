# Dashboard Aggregation Optimization Analysis

## Original Code Profile

The function `ingresos_por_propiedad_mes` aggregates payment data (~2000 pagos, ~50 propiedades, 24 months) on every dashboard page load.

## Bottlenecks Identified

### 1. Per-iteration String allocation via `format!` (HIGH impact)

```rust
let mes = pago.fecha_pago.unwrap().format("%Y-%m").to_string();
```

This allocates a new heap `String` for every qualifying pago (~2000 times). The `chrono` formatting machinery is also non-trivial — it parses the format string, dispatches formatters, and writes to a buffer. This is the dominant cost in the hot loop.

### 2. String-keyed inner HashMap (MEDIUM impact)

Using `HashMap<String, f64>` for the inner map means:
- Every `.entry(mes)` call hashes a 7-byte string (heap-allocated).
- The final sort compares strings lexicographically.

With only ~24 unique months, this is wasteful.

### 3. No capacity pre-allocation (LOW impact)

`HashMap::new()` starts with 0 capacity and resizes as entries are added. With known cardinality (~50 propiedades), pre-allocation avoids 2-3 reallocations.

### 4. Filter + unwrap pattern (MINOR)

```rust
.filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
// later:
pago.fecha_pago.unwrap()
```

The filter checks `is_some()` and then `unwrap()` re-checks. A single `if let` / `let-else` is cleaner and marginally faster (one fewer branch).

## Optimizations Applied

| # | Change | Expected Impact |
|---|--------|----------------|
| 1 | Replace `format("%Y-%m")` with integer key `year*12 + month` | Eliminates ~2000 String allocations per call |
| 2 | Inner map keyed by `u32` instead of `String` | Faster hashing (single u32 vs 7-byte string) |
| 3 | `sort_unstable_by_key` on integer | Faster sort, no string comparison |
| 4 | Convert to `"YYYY-MM"` string only in output phase | Only ~1200 conversions (50 props × 24 months) vs ~2000 |
| 5 | `HashMap::with_capacity(64)` for outer map | Avoids reallocations |
| 6 | `let Some(fecha) = ... else { continue }` | Single branch, no unwrap, idiomatic Rust 2024 |

## Expected Performance Improvement

For the production dataset (~2000 pagos):
- **Allocation reduction**: ~2000 String allocations eliminated from the hot loop. Only ~1200 remain in the cold output-formatting phase.
- **Hashing**: u32 hashing is ~3-5x faster than 7-byte string hashing.
- **Sorting**: Integer comparison is ~2x faster than string comparison for "YYYY-MM" keys.
- **Overall estimate**: 2-4x faster for the hot path, depending on allocator pressure.

## Correctness

The output type and semantics are identical: `HashMap<Uuid, Vec<(String, f64)>>` with months sorted chronologically. The integer encoding `year*12 + month` preserves chronological ordering, so sorting by key produces the same result as lexicographic sort on "YYYY-MM" strings.

## What Was NOT Changed

- The public API signature remains identical.
- The `Pago` struct is unchanged.
- No new dependencies are required (`chrono::Datelike` is already available via `chrono`).
- No parallelism was introduced — at ~2000 items, thread overhead would likely exceed gains.
