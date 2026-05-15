# Overlap Detection Performance Analysis

## Question
The overlap detection is O(n²). During normal use n=5-15 but during bulk import n can be 200. Should we optimize it?

## Answer: Yes, but the optimization is straightforward and low-risk.

### Current Complexity

The current implementation does pairwise comparison: O(n²) comparisons with constant-time overlap checks per pair.

| n | Comparisons (n*(n-1)/2) |
|---|---|
| 5 | 10 |
| 15 | 105 |
| 200 | 19,900 |

### Is 19,900 comparisons a problem?

For a single call with n=200, 19,900 comparisons of simple date ranges is **not a performance bottleneck** in absolute terms — it completes in microseconds on modern hardware. The comparisons are cheap (two integer comparisons for dates, a string equality check, and a UUID equality check).

However, there are good reasons to optimize anyway:

1. **During bulk import, this function may be called repeatedly** — once per contract being validated. If you're importing 200 contracts and validating after each insertion, you get O(n³) total work.
2. **The optimized version is equally readable** and actually expresses the intent more clearly.
3. **It filters by `propiedad_id` first**, which the current version checks inside the inner loop — grouping by property first eliminates most comparisons entirely.

### Recommended Approach

Sort contracts by `(propiedad_id, fecha_inicio)`, then scan adjacent pairs within each property group. This gives O(n log n) from the sort, plus O(n) for the scan.

For the bulk import case (n=200), if contracts span multiple properties (likely), the per-group sizes are much smaller, making the scan trivially fast.

### Key Insight

The biggest win isn't sort-vs-pairwise — it's **grouping by `propiedad_id` first**. The current code compares contracts across different properties (wasted work). A `HashMap` grouping eliminates cross-property comparisons entirely.

After grouping, within each property:
- If groups are small (5-15), pairwise is fine.
- If a single property has 200 contracts, sort + sweep is better.

The hybrid approach (group first, then sort+sweep within each group) handles both cases optimally.

### Correctness Note

**Important**: Sort + adjacent-pair scan only detects overlaps between consecutive intervals. It does NOT detect all overlapping pairs when intervals can overlap non-adjacent entries (e.g., a long interval overlapping multiple short ones). 

For correctness, we need a **sweep-line algorithm**: sort by start date, maintain a set of "active" intervals, and check each new interval against all currently active ones. This is O(n log n) for the sort + O(n * k) where k is the average number of concurrent active intervals (typically small for real estate contracts).

## Verdict

Optimize with: group by `propiedad_id` → sort by `fecha_inicio` → sweep-line within each group. The code is cleaner, correct for all cases, and handles the bulk import scenario efficiently.
