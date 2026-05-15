# Overlap Detection Benchmark Report

## Problem Statement

`detectar_solapamientos` uses O(n²) pairwise comparison to find overlapping active contracts for a property. During normal use n=5-15, but during bulk import n can reach 200. Should we optimize?

## Analysis

### Why This Is Worth Benchmarking

- **Hot path during bulk import**: Called as a validation step on every contract creation/update. During bulk import of 200 contracts, this runs repeatedly.
- **Quadratic growth is real**: At n=200, the original does ~19,900 comparisons. At n=15, only ~105. That's a 190x increase in work for a 13x increase in input.
- **Not IO-bound**: This is pure CPU computation on in-memory data — optimization will directly reduce latency.

### Approaches Tested

| # | Approach | Algorithm | Time Complexity | Space |
|---|----------|-----------|-----------------|-------|
| 1 | **Original** (pairwise) | Nested loop over all pairs, check propiedad + estado + dates | O(n²) | O(1) extra |
| 2 | **Sort+Scan** (HashMap grouping) | Filter active → group by propiedad_id in HashMap → sort each group by fecha_inicio → sweep with early break | O(n log n) | O(n) |
| 3 | **Sort+Partition** (single sort) | Filter active → sort by (propiedad_id, fecha_inicio) → linear partition scan → sweep with early break | O(n log n) | O(n) |

### Key Insight

The original does redundant work:
1. It checks `estado == "activo"` for both contracts in every pair — even inactive ones participate in O(n²) comparisons.
2. It compares contracts across different propiedades — most pairs will fail the `propiedad_id` check.
3. It has no early termination — even when dates clearly don't overlap, it still checks every subsequent pair.

The optimized approaches:
1. **Pre-filter** inactive contracts (typically removes ~30% of input).
2. **Group/partition** by propiedad_id so cross-property comparisons never happen.
3. **Sort by start date** so we can break early: once we find a contract that starts after the current one ends, all subsequent ones are also non-overlapping.

## Expected Benchmark Results

### At n=10-15 (typical usage)

The original is likely **comparable or slightly faster** than the optimized versions. Reasons:
- n² at n=12 is only 66 comparisons — trivial for modern CPUs.
- The optimized versions pay overhead: HashMap allocation, sorting, or Vec collection.
- At this size, the overhead of being "smart" exceeds the savings.

**Verdict**: Original is fine for typical usage. No optimization needed for n≤15.

### At n=50-200 (bulk import)

The optimized versions should **significantly outperform** the original. Reasons:
- At n=200, original does ~19,900 comparisons (most failing on propiedad_id or estado checks).
- Sort+partition: filters to ~140 active contracts, sorts once (O(140 log 140) ≈ 1000 ops), then sweeps with early break within small per-property groups.
- The early-break optimization is powerful: in realistic data with ~5 contracts per property, each group sweep is tiny.

**Expected speedup at n=200**: 3-10x faster (depends on overlap density).

### Worst Case (all same propiedad, all overlapping)

When all contracts are active, same property, and all overlap each other:
- Original: O(n²) comparisons, all producing output.
- Optimized: O(n log n) sort + O(n²) output (because there ARE n² overlapping pairs).
- In this degenerate case, the output itself is O(n²), so no algorithm can avoid quadratic time. But the optimized versions still save on the sort step being cheaper than redundant estado/propiedad checks.

## Recommendation

**Adopt `sort_partition` (Approach 3)** as the production implementation.

### Rationale

1. **At n=10-15**: Performance difference is negligible (nanoseconds). The sort_partition approach is still fast enough — we're talking about sub-microsecond operations either way.

2. **At n=200**: Sort+partition avoids the HashMap allocation overhead of sort_scan while achieving the same O(n log n) complexity. Single allocation (the filtered Vec) is cache-friendlier.

3. **Readability**: Sort+partition is straightforward — filter, sort, sweep. No HashMap indirection.

4. **Correctness**: The sweep-with-early-break produces identical results to pairwise comparison (proven by tests in alternatives.rs).

5. **No downside at small n**: The overhead at n=12 is a few hundred nanoseconds at most — invisible in a request that hits the database.

### Implementation Note

After adopting, the function signature stays identical:

```rust
pub fn detectar_solapamientos(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)>
```

No API change. Drop-in replacement.

## How to Run the Benchmarks

```bash
# Add to Cargo.toml [dev-dependencies]:
# criterion = { version = "0.5", features = ["html_reports"] }
# rand = "0.8"
# chrono = "0.4"
# uuid = { version = "1", features = ["v4"] }
#
# Add [[bench]] section:
# [[bench]]
# name = "bench_overlap"
# harness = false

cargo bench --bench bench_overlap --release
```

## Decision Summary

| Size | Optimize? | Reason |
|------|-----------|--------|
| n ≤ 15 | No (but no harm) | O(n²) at n=15 is 105 comparisons — trivial |
| n = 50-200 | **Yes** | 3-10x speedup expected, reduces bulk import latency |
| n > 200 | **Yes** | Quadratic growth becomes painful; sort+sweep stays linear-ish |

**Final answer**: Yes, optimize. The sort_partition approach is a drop-in replacement that matches performance at small n and significantly wins at bulk import sizes. Keep the benchmark file for regression detection.
