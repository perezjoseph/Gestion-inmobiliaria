# Payment Search Benchmark Analysis

## Question

Is pre-indexing by `contrato_id` (HashMap) faster than the current linear scan for `buscar_pagos`?

## Answer: Yes — ~23x faster for the dominant query pattern

Pre-indexing with a `HashMap<Uuid, Vec<usize>>` by `contrato_id` delivers a **23x speedup** for the most common query pattern (filter by `contrato_id`, which represents ~80% of all queries).

## Benchmark Results (5000 payments, 100 contracts)

| Scenario | Linear Scan | Indexed HashMap | Speedup |
|----------|-------------|-----------------|---------|
| Filter by contrato_id only | 24.97 µs | 1.10 µs | **23x** |
| contrato_id + estado | 26.82 µs | 1.17 µs | **23x** |
| contrato_id + date range | 28.10 µs | 0.60 µs | **47x** |
| No contrato_id (fallback) | 76.96 µs | 70.09 µs | ~1.1x |

### Index Construction Cost

| Operation | Time |
|-----------|------|
| Build index (5000 pagos) | ~1.05 ms |

### Scaling Behavior (filter by contrato_id)

| Dataset Size | Linear Scan | Indexed | Speedup |
|--------------|-------------|---------|---------|
| 1,000 | 6.34 µs | 1.20 µs | 5x |
| 2,500 | 13.11 µs | 1.17 µs | 11x |
| 5,000 | 28.17 µs | 1.22 µs | 23x |
| 10,000 | 52.63 µs | 1.20 µs | 44x |

Key observation: **Linear scan scales O(n) while indexed lookup stays constant O(1)**. The indexed approach becomes more valuable as the dataset grows.

## Recommendation

**Adopt the indexed approach.** The tradeoffs are favorable:

- **Construction cost is negligible**: ~1ms to build the index for 5000 payments. This is a one-time cost when loading the payments list, amortized over many queries (pagination, filtering, sorting all trigger re-queries).
- **Memory overhead is minimal**: One `HashMap<Uuid, Vec<usize>>` storing indices — roughly `100 entries × 50 indices × 8 bytes = ~40KB` for the production dataset.
- **Fallback is no worse**: When no `contrato_id` filter is provided (~20% of queries), the indexed version falls back to linear scan with negligible overhead (skips the contrato_id comparison).
- **Scales well**: As the dataset grows, the indexed version maintains constant-time lookup while linear scan degrades linearly. At 10K payments the speedup reaches 44x.

## Implementation Notes

The recommended implementation:

1. Build `PagoIndex` once when the payments service loads data (or on cache invalidation).
2. Use `HashMap<Uuid, Vec<usize>>` keyed by `contrato_id` pointing to indices in the payments vec.
3. When `contrato_id` filter is present: O(1) HashMap lookup → iterate only matching subset (~50 payments per contract vs 5000 total).
4. When `contrato_id` filter is absent: fall back to linear scan (same as current behavior).
5. Apply remaining filters (estado, date range, referencia) only to the narrowed subset.

This is a pure read-path optimization with no impact on write paths. The index should be rebuilt when payments are added/modified (which is infrequent compared to reads on the list page).

## Why Not a BTreeMap by Date?

A secondary BTreeMap index by `fecha_vencimiento` was considered but not implemented because:

- Date range queries are the *second* most common filter and almost always combined with `contrato_id`.
- Once narrowed to ~50 payments for a single contract, a linear scan over dates is already sub-microsecond.
- Adding a BTreeMap would increase memory and construction cost for marginal gain on an already-fast path.

If date-only queries (without contrato_id) become a hot path in the future, a BTreeMap index would be worth revisiting.
