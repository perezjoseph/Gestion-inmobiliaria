# Overlap Detection Performance Analysis

## Question

The `detectar_solapamientos` function uses O(n²) pairwise comparison. During normal use n=5-15, but during bulk import n can reach 200. Should we optimize it?

## Implementations Tested

1. **Original O(n²)**: Brute-force pairwise comparison of all contracts
2. **Optimized (sort+scan with HashMap grouping)**: Filter activos → group by propiedad_id → sort each group by fecha_inicio → scan with early termination
3. **Sort-only**: Filter activos → sort by fecha_inicio → scan (no HashMap, for single-propiedad case)

## Benchmark Results

### Normal Use (n=5-15, single propiedad, all active)

| n  | Original O(n²) | Optimized (sort+scan) | Sort-only |
|----|---------------:|----------------------:|----------:|
| 5  | **451 ns**     | 1,152 ns              | 694 ns    |
| 10 | **842 ns**     | 1,998 ns              | 1,115 ns  |
| 15 | **1,283 ns**   | 2,337 ns              | 1,485 ns  |

**Winner at normal scale: Original O(n²)**

The original is 2-2.5x faster at small n. The HashMap allocation and sorting overhead dominate when n is small. Even sort-only is ~50% slower than the simple nested loop.

### Bulk Import (n=200)

| Scenario | Original O(n²) | Optimized (sort+scan) | Sort-only |
|----------|---------------:|----------------------:|----------:|
| Single propiedad (worst case) | 218 µs | **161 µs** | 165 µs |
| Multi propiedad (20 props, 80% active) | 36.4 µs | **19.8 µs** | 35.9 µs |

**Winner at bulk scale: Optimized (sort+scan)**

- Single propiedad: optimized is **1.35x faster** (26% reduction)
- Multi propiedad: optimized is **1.82x faster** (45% reduction)

### Scaling Behavior (single propiedad, all active)

| n   | Original O(n²) | Optimized (sort+scan) | Ratio (orig/opt) |
|-----|---------------:|----------------------:|-----------------:|
| 10  | 985 ns         | 2,021 ns              | 0.49x (orig wins) |
| 50  | 10.4 µs        | 11.5 µs              | 0.90x (roughly equal) |
| 100 | 40.2 µs        | 38.1 µs              | 1.05x (crossover) |
| 200 | 206 µs         | 160 µs               | 1.29x (opt wins) |
| 500 | 4,114 µs       | 3,418 µs             | 1.20x (opt wins) |

**Crossover point: ~n=80-100** — below this the original wins, above it the optimized version wins.

## Recommendation: Don't Optimize

**Keep the original O(n²) implementation.** Here's why:

### 1. Normal use dominates (n=5-15)
The original is 2-2.5x faster in the common case. This is the hot path — every contract creation/update triggers this validation.

### 2. Bulk import gains are modest
At n=200, the optimized version saves ~57 µs (single prop) or ~16 µs (multi prop). These are **microsecond-level** differences. In a bulk import of 200 contracts, the database I/O will be orders of magnitude slower (milliseconds per query). The overlap check is not the bottleneck.

### 3. Absolute times are tiny
Even the "slow" O(n²) at n=200 takes only **218 µs** — well under 1ms. This is negligible compared to any network or database operation in the validation pipeline.

### 4. Complexity cost
The optimized version adds HashMap allocation, grouping logic, and sorting. More code to maintain, more potential for subtle bugs (e.g., result ordering differences), and harder to reason about correctness.

### 5. The real optimization for bulk import
If bulk import performance matters, the optimization should happen at the **batch level** (fewer DB round-trips, transaction batching) not at the overlap detection level. The O(n²) check on 200 items is not the bottleneck.

## When WOULD optimization be warranted?

- If n regularly exceeds 500+ (4ms+ per check)
- If the function is called in a tight loop without I/O between calls
- If profiling shows this function as a measurable % of request latency

None of these apply to the current use case.

## Verdict

**No optimization needed.** The O(n²) approach is simpler, faster for the common case, and the bulk import scenario is still well within acceptable latency. The ~200µs worst case is noise compared to the database operations surrounding it.
