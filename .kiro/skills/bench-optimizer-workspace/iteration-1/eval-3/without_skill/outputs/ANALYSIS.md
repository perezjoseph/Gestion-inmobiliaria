# Payment Search Benchmark Analysis

## Question

Is pre-indexing by `contrato_id` (HashMap) faster than the current linear scan for `buscar_pagos`?

## Answer: Yes — ~30x faster for the dominant query pattern

Pre-indexing with a `HashMap<Uuid, Vec<usize>>` by `contrato_id` delivers a **30x speedup** for the most common query (filter by `contrato_id`, which represents ~80% of all queries).

## Benchmark Results (5000 payments, 100 contracts)

| Scenario | Linear Scan | Indexed HashMap | Speedup |
|----------|-------------|-----------------|---------|
| Filter by contrato_id only | 39.2 µs | 1.29 µs | **30x** |
| contrato_id + estado | 28.5 µs | 1.27 µs | **22x** |
| contrato_id + date range | 27.4 µs | 0.81 µs | **34x** |
| No contrato_id (fallback) | 75.8 µs | 65.3 µs | 1.2x |

### Index Construction Cost

| Operation | Time |
|-----------|------|
| Build index (5000 pagos) | ~1.0 ms |

### Scaling Behavior (filter by contrato_id)

| Dataset Size | Linear Scan | Indexed | Speedup |
|--------------|-------------|---------|---------|
| 1,000 | 6.0 µs | 1.17 µs | 5x |
| 2,500 | 13.8 µs | 1.19 µs | 12x |
| 5,000 | 27.2 µs | 1.28 µs | 21x |
| 10,000 | 55.7 µs | 1.22 µs | 46x |

Key observation: **Linear scan scales O(n) while indexed lookup stays constant O(1)**. The indexed approach becomes more valuable as the dataset grows.

## Recommendation

**Adopt the indexed approach.** The tradeoffs are favorable:

- **Construction cost is negligible**: 1ms to build the index for 5000 payments. This is a one-time cost when loading the payments list, amortized over many queries (pagination, filtering, sorting all trigger re-queries).
- **Memory overhead is minimal**: One `HashMap<Uuid, Vec<usize>>` storing indices — roughly `100 entries × 50 indices × 8 bytes = ~40KB` for the production dataset.
- **Fallback is no worse**: When no `contrato_id` filter is provided (~20% of queries), the indexed version falls back to linear scan with negligible overhead (~14% faster due to avoiding the contrato_id comparison).
- **Scales well**: As the dataset grows, the indexed version maintains constant-time lookup while linear scan degrades linearly.

## Implementation Notes

The recommended implementation:

1. Build `PagoIndex` once when the payments service loads data (or on cache invalidation).
2. Use `HashMap<Uuid, Vec<usize>>` keyed by `contrato_id` pointing to indices in the payments vec.
3. When `contrato_id` filter is present: O(1) HashMap lookup → iterate only matching subset.
4. When `contrato_id` filter is absent: fall back to linear scan (same as current behavior).

This is a pure read-path optimization with no impact on write paths. The index should be rebuilt when payments are added/modified (which is infrequent compared to reads on the list page).
