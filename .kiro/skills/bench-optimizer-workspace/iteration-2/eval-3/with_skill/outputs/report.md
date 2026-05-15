# Payment Search Benchmark Report

## Question

Is pre-indexing by `contrato_id` faster than the current linear scan for `buscar_pagos`?

## Answer: Yes — adopt the HashMap index

The HashMap index is **~40-45x faster** for the dominant query pattern (filter by `contrato_id`,
which represents 80% of production queries). The improvement is large enough to justify the
added complexity.

## Measured Results

### Primary benchmark: filter by contrato_id (5000 pagos, 200 contratos)

| Approach | Mean time | Speedup vs linear |
|----------|-----------|-------------------|
| Linear scan | 32.05 µs | baseline |
| HashMap index (indices) | 708 ns | **~45x faster** |
| Ref index (pointers) | 691 ns | **~46x faster** |

### Combined filter: contrato_id + estado

| Approach | Mean time | Speedup vs linear |
|----------|-----------|-------------------|
| Linear scan | 26.05 µs | baseline |
| HashMap index | 254 ns | **~103x faster** |
| Ref index | 227 ns | **~115x faster** |

### Date range only (no contrato_id — index provides no benefit)

| Approach | Mean time | Notes |
|----------|-----------|-------|
| Linear scan | 56.69 µs | baseline |
| HashMap index (fallback) | 57.37 µs | Same — correctly falls back to linear |

### Index build cost

| Index type | Build time |
|------------|-----------|
| HashMap (indices) | 332 µs |
| Ref index (pointers) | 309 µs |

### Scaling behavior (linear scan vs hashmap index, filter by contrato_id)

| Dataset size | Linear scan | HashMap index | Speedup |
|-------------|-------------|---------------|---------|
| 500 | 3.50 µs | 663 ns | ~5x |
| 1000 | 5.08 µs | 643 ns | ~8x |
| 2500 | 12.12 µs | 663 ns | ~18x |
| 5000 | 25.42 µs | 605 ns | ~42x |

The index lookup time stays constant (~600-700ns) regardless of dataset size, while
linear scan grows linearly. This confirms O(1) lookup vs O(n) scan behavior.

## Recommendation

**Adopt the HashMap index approach (`PagoIndex` with `HashMap<Uuid, Vec<usize>>`).**

Rationale:
- 45x speedup on the dominant query pattern (80% of queries filter by contrato_id)
- Index build cost (332µs) is amortized across many queries — pays for itself after ~10 lookups
- No regression for non-indexed queries (falls back to linear scan)
- Simple implementation: one HashMap, built on data load/change
- The ref index is marginally faster but ties the index lifetime to the data, making it
  harder to use in practice (e.g., behind an Arc<RwLock>). The indices approach is more flexible.

## Context: Is this worth optimizing?

- This function is called on every page load of the payments list (hot path: confirmed)
- At 5000 pagos, the linear scan takes ~32µs — not terrible in absolute terms
- But the DB query + serialization likely takes 5-20ms, so 32µs is ~0.2-0.6% of request time
- However, the index makes it essentially free (~700ns), and the implementation is simple
- **Verdict: worth it.** The index is simple, the speedup is massive, and it prevents
  the function from becoming a bottleneck as the dataset grows.

## Build cost amortization

The index costs ~332µs to build. At 700ns per lookup, the break-even point is:
- 332µs / (32µs - 0.7µs) ≈ **11 queries** to recoup the build cost

Since the payments page is loaded frequently and the data changes infrequently (new payments
are added maybe a few times per day), building the index once and invalidating on write is
clearly worthwhile.

## Cargo bench output

```
filter_by_contrato_id/linear_scan
                        time:   [28.817 µs 32.050 µs 36.084 µs]
filter_by_contrato_id/hashmap_index
                        time:   [648.43 ns 708.18 ns 779.42 ns]
filter_by_contrato_id/ref_index
                        time:   [647.75 ns 691.51 ns 742.74 ns]

contrato_id_plus_estado/linear_scan
                        time:   [24.453 µs 26.050 µs 28.009 µs]
contrato_id_plus_estado/hashmap_index
                        time:   [224.59 ns 253.92 ns 292.48 ns]
contrato_id_plus_estado/ref_index
                        time:   [213.90 ns 227.40 ns 244.30 ns]

date_range_only/linear_scan
                        time:   [55.437 µs 56.685 µs 58.358 µs]
date_range_only/hashmap_index
                        time:   [55.567 µs 57.371 µs 59.677 µs]

index_build_cost/build_hashmap_index
                        time:   [301.20 µs 332.43 µs 368.95 µs]
index_build_cost/build_ref_index
                        time:   [291.21 µs 308.62 µs 330.59 µs]

scaling_by_size/linear_scan/500
                        time:   [3.2543 µs 3.4952 µs 3.7739 µs]
scaling_by_size/hashmap_index/500
                        time:   [634.37 ns 663.20 ns 702.11 ns]
scaling_by_size/linear_scan/1000
                        time:   [5.0716 µs 5.0807 µs 5.0910 µs]
scaling_by_size/hashmap_index/1000
                        time:   [619.70 ns 642.87 ns 672.90 ns]
scaling_by_size/linear_scan/2500
                        time:   [11.875 µs 12.117 µs 12.450 µs]
scaling_by_size/hashmap_index/2500
                        time:   [627.83 ns 662.62 ns 707.32 ns]
scaling_by_size/linear_scan/5000
                        time:   [24.572 µs 25.424 µs 26.492 µs]
scaling_by_size/hashmap_index/5000
                        time:   [594.65 ns 605.04 ns 619.38 ns]
```
