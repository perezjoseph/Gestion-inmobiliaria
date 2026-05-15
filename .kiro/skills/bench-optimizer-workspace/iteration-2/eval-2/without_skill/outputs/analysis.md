# Overlap Detection Performance Analysis

## Question

The `detectar_solapamientos` function uses O(n²) pairwise comparison. Normal use has n=5-15, but bulk import can hit n=200. Should we optimize?

## Analysis

### Current Complexity

- **Normal case (n=15):** ~105 comparisons. Trivial — sub-microsecond.
- **Bulk import (n=200):** ~19,900 comparisons. Still very fast — likely under 100μs on modern hardware.

### Is O(n²) a problem at n=200?

**No.** 19,900 comparisons with simple field checks (two UUID comparisons, two string comparisons, two date comparisons) is well within "instant" territory. Each comparison involves:
- 2 UUID equality checks (128-bit compare)
- 2 string equality checks (short strings: "activo")
- 2 date comparisons (NaiveDate is just an i32 internally)

This is cache-friendly, branch-predictor-friendly work. Even pessimistically at 50ns per iteration, n=200 takes ~1ms. In practice it's much less.

### When would optimization matter?

An O(n log n) sort-then-scan approach would only show measurable benefit at n > ~1000-2000 for this kind of lightweight comparison. The sort itself has overhead (allocation, moves of the `Contrato` struct or index array) that makes it slower than brute-force for small n.

### Verdict: Don't optimize

The O(n²) approach is:
1. **Fast enough** — even at n=200, it's sub-millisecond
2. **Correct** — finds ALL overlapping pairs, not just adjacent ones after sorting
3. **Simple** — easy to read, easy to verify against the business invariant
4. **Maintainable** — no sorting assumptions to document or break

### Important correctness note

A sort-by-start-date + adjacent-scan approach would be **incorrect** here anyway. Sorting by `fecha_inicio` and only checking adjacent pairs misses non-adjacent overlaps. For example:

```
Contract A: Jan 1 - Dec 31
Contract B: Feb 1 - Mar 31
Contract C: Apr 1 - May 31
```

Sorted by start: A, B, C. Adjacent scan finds (A,B) and (B,C) — but misses (A,C), which also overlaps. You'd need a sweep-line algorithm to handle this correctly, which adds complexity for no practical gain at n=200.

## Recommendation

**Keep the current implementation as-is.** The code is correct, readable, and fast enough for the stated workload. The O(n²) label sounds scary but the actual wall-clock time at n=200 is negligible.

If bulk import sizes ever grow to n > 1000, revisit with a sweep-line algorithm (sort by start, maintain active set, O(n log n) with correct overlap detection). But that's a premature optimization today.

## Code Improvements (non-performance)

The current code could benefit from minor idiomatic improvements per the project's code style guidelines (iterator chains over loops, `&str` comparison), but these don't affect performance. See `optimized.rs` for a lightly cleaned-up version that follows project conventions without changing the algorithm.
