# Dashboard Aggregation Optimization

## Problem

`ingresos_por_propiedad_mes` is called on every page load. With ~2000 pagos, ~50 propiedades, and 24 months of data, the function needs to be as fast as possible.

## Bottlenecks Identified

| # | Issue | Impact |
|---|-------|--------|
| 1 | `format!("%Y-%m").to_string()` per pago | ~1600 String allocations in the hot loop (80% are "pagado") |
| 2 | `HashMap<String, f64>` inner map | String hashing is expensive vs integer hashing |
| 3 | Separate `.sort_by()` pass per propiedad | Extra O(m log m) per propiedad after accumulation |
| 4 | No capacity pre-allocation | Repeated HashMap resizing |

## Optimizations Applied

1. **Integer month encoding**: Replace `format!("%Y-%m")` with `encode_month(date) -> u32` (`year * 12 + month`). This eliminates all String allocations in the hot loop. Strings are only created at the end for the ~1200 unique (propiedad, month) pairs in the output.

2. **BTreeMap for inner map**: The inner map uses `BTreeMap<u32, f64>` instead of `HashMap<String, f64>`. This gives sorted iteration for free, eliminating the separate sort pass. BTreeMap with u32 keys is also cache-friendlier than HashMap with String keys for small cardinalities (~24 entries per propiedad).

3. **Pre-allocated outer HashMap**: `HashMap::with_capacity(50)` avoids resizing for the expected ~50 propiedades.

4. **Deferred string conversion**: `decode_month(u32) -> String` is called only once per unique (propiedad, month) pair in the final output phase, not per pago.

## Expected Improvement

- **Allocation reduction**: From ~1600 String allocations (one per paid pago) down to ~1200 (one per unique output pair). The hot loop is now allocation-free.
- **Hashing**: u32 hashing is significantly cheaper than String hashing.
- **Sorting**: Eliminated entirely (BTreeMap maintains order).
- **Estimated speedup**: 2-4x for the production dataset size, primarily from eliminating per-pago allocations.

## How to Benchmark

```bash
cd outputs/
cargo bench
```

The benchmark compares both implementations with the production-scale dataset (2000 pagos, 50 propiedades).

## Correctness

The output type and semantics are identical: `HashMap<Uuid, Vec<(String, f64)>>` with months sorted chronologically. The optimization is a drop-in replacement.
