# Overlap Detection Performance Analysis

## Question

The current `detectar_solapamientos` is O(n²) pairwise comparison. With n=5-15 normally and n=200 during bulk import, should we optimize?

## Answer: No, keep the O(n²) approach.

### Reasoning

**The numbers don't justify optimization:**

| n | Comparisons (n²/2) | Estimated time |
|---|---|---|
| 15 | 105 | ~1 µs |
| 200 | 19,900 | ~20-50 µs |

At n=200, you're doing ~20,000 comparisons. Each comparison is a few field checks (two UUID equality checks, two string comparisons, two date comparisons). On modern hardware this completes in tens of microseconds — well below any threshold where a user or system would notice.

**The "optimized" alternative isn't actually better here:**

A sort-then-scan approach (O(n log n)) would:
1. Require filtering to only `activo` contracts first
2. Require grouping by `propiedad_id` first
3. Sort each group by `fecha_inicio`
4. Scan adjacent pairs

But adjacent-pair scanning **only works for non-overlapping detection in sorted intervals when each interval can overlap at most one neighbor**. For general interval overlap detection (where one contract could overlap multiple others), you still need a sweep-line algorithm, which adds complexity.

**The current code is correct and handles the general case** — it finds ALL overlapping pairs, not just adjacent ones. A sort-based approach that only checks adjacent pairs would **miss overlaps** when contract A overlaps contracts C and D but not B (where B sits between A and C in sorted order).

**Complexity cost of "optimizing":**
- More code to maintain
- Subtle correctness bugs (missing non-adjacent overlaps)
- Negligible performance gain (microseconds saved)
- Violates YAGNI — the current code works fine at n=200

### When would optimization matter?

If n regularly exceeded ~10,000, an interval-tree or sweep-line approach would be warranted. At n=200, the O(n²) approach is the right choice: simple, correct, and fast enough.

### Recommendation

Keep the current implementation as-is. The O(n²) pairwise comparison is:
- **Correct**: finds all overlapping pairs, not just adjacent
- **Fast enough**: ~20K comparisons at n=200 is trivial
- **Simple**: easy to read, easy to verify, no subtle bugs
- **Maintainable**: any developer can understand it immediately

If bulk import performance becomes a concern, the bottleneck will be database I/O, not this in-memory comparison.
