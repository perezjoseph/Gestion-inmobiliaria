# Dashboard Aggregation Optimization Analysis

## Problem

`ingresos_por_propiedad_mes` is called on every dashboard page load. It aggregates total income per property per month from ~2000 payment records across ~50 properties over 24 months.

## Original Implementation Bottlenecks

1. **String allocation in hot loop**: `fecha_pago.unwrap().format("%Y-%m").to_string()` allocates a new heap String for every qualifying pago (~667 allocations per call, assuming 1/3 are "pagado").
2. **String hashing**: The inner HashMap uses `String` keys, which requires hashing variable-length data on every insert/lookup.
3. **String comparison during sort**: Final sort compares "YYYY-MM" strings lexicographically (7 bytes each).
4. **No capacity hints**: Both outer and inner HashMaps start empty and rehash as they grow.

## Optimizations Applied

| # | Change | Why it helps |
|---|--------|-------------|
| 1 | Replace `String` month key with `u32` (year×12 + month0) | Eliminates ~667 heap allocations per call. u32 hashing is a single instruction vs. variable-length string hashing. |
| 2 | `HashMap::with_capacity(50)` for outer map | Avoids 3-4 rehashes as propiedades are inserted. |
| 3 | `HashMap::with_capacity(24)` for inner maps | Avoids rehashes for month buckets (max 24 months). |
| 4 | `sort_unstable_by_key` on u32 | Faster than `sort_by` on String: no allocations, integer comparison, and unstable sort avoids extra moves. |
| 5 | Format strings only in final phase | Converts u32→"YYYY-MM" only for the ~1200 final output entries, not during accumulation. |
| 6 | Early `continue` instead of `.filter().unwrap()` | Avoids iterator adapter overhead and the unwrap; branch predictor friendly. |

## Expected Performance Improvement

For the production dataset (2000 pagos, 50 propiedades, 24 months):

- **Allocation reduction**: ~667 String allocations eliminated from the hot loop.
- **Hashing speedup**: u32 hash is ~3-5× faster than String hash per operation.
- **Sort speedup**: Integer comparison vs. string comparison on 24-element vectors.
- **Overall estimate**: 30-50% faster for this dataset size, primarily from eliminating allocator pressure.

## Correctness

The optimized version produces identical output:
- Same filtering (estado == "pagado" && fecha_pago.is_some())
- Same grouping (by propiedad_id)
- Same accumulation (sum of monto per month)
- Same output format (HashMap<Uuid, Vec<(String, f64)>> sorted by month ascending)
- The u32 encoding `year * 12 + month0` preserves chronological order, so sorting by key gives the same result as lexicographic sort on "YYYY-MM".

## How to Benchmark

```bash
cd outputs/
cargo bench
```

The benchmark compares both implementations on a synthetic dataset matching production characteristics (2000 pagos, 50 propiedades, 24 months spread).

## Further Optimization Opportunities (not implemented)

- **Interning estado**: If `Pago.estado` were an enum instead of String, the filter check would be a single integer comparison instead of string equality.
- **Pre-sorted input**: If pagos arrive sorted by propiedad_id, a single pass with group-by would avoid HashMap overhead entirely.
- **Caching**: Since this runs on every page load, caching the result and invalidating on pago mutations would eliminate recomputation entirely. This is an architectural change beyond the scope of this optimization.
