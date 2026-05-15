# Overlap Detection Performance Analysis

## Summary

**Recommendation: Keep the O(n²) approach.** The optimization is not worth the added complexity for this use case.

## Reasoning

### Dataset Size Analysis

| Scenario | n | Comparisons (n²/2) | Estimated Time |
|----------|---|---------------------|----------------|
| Normal use | 5-15 | 10-105 | < 1 µs |
| Bulk import (worst case) | 200 | 19,900 | ~10-50 µs |

Even at n=200, ~20,000 comparisons of simple date/string checks complete in microseconds on modern hardware. This is well below any threshold where a user or system would notice.

### Why Sorting Doesn't Help Here

The suggested optimization (sort by `fecha_inicio`, scan adjacent pairs) has a critical flaw: **it only detects overlaps between adjacent intervals**. Non-adjacent intervals can still overlap. For example:

```
Contract A: Jan 1 - Dec 31
Contract B: Feb 1 - Mar 31
Contract C: Jun 1 - Jul 31
```

After sorting by `fecha_inicio`: A, B, C. Adjacent-pair scanning finds A↔B and B↔C, but **misses A↔C**.

To correctly detect all overlaps with a sort-based approach, you'd need a sweep-line algorithm:
1. Sort by `fecha_inicio` — O(n log n)
2. Maintain an active set, removing contracts whose `fecha_fin` has passed — O(n log n) with a heap

This is O(n log n) but adds significant implementation complexity (active set management, heap operations) for a gain that's invisible at n=200.

### Additional Considerations

1. **The filter on `propiedad_id`**: In practice, contracts are already scoped to a single propiedad (the function comment says "for a given propiedad"). If the caller pre-filters, n is even smaller.

2. **The filter on `estado == "activo"`**: Most contracts in a bulk import won't all be active simultaneously. The effective comparison count is lower than the theoretical n².

3. **Correctness risk**: The current implementation is trivially correct. A sweep-line algorithm is harder to verify and maintain.

4. **Where time actually goes during bulk import**: Database I/O, transaction management, and validation queries dominate. The in-memory overlap check is noise.

## When to Reconsider

Optimize if:
- n regularly exceeds 10,000+ (not the case here)
- Profiling shows this function as a hotspot (unlikely given the I/O-bound context)
- The function is called in a tight loop without pre-filtering by propiedad_id

## Conclusion

At n=200, the O(n²) approach completes in microseconds. The code is simple, correct, and easy to maintain. The sort-based alternative adds complexity for no measurable user-facing benefit. Keep it as-is.
