# Decision: Don't Optimize

**Keep the HashMap approach.** The 7µs difference is irrelevant in this context.

## Reasoning

Your endpoint has 50ms (50,000µs) of DB latency. The entire grouping operation — either approach — accounts for less than 0.1% of request time. The sort+scan's 15% advantage translates to saving 7µs per request, which is unmeasurable in end-to-end latency.

The sort+scan approach also requires cloning the input Vec, which adds complexity and allocation pressure for zero user-visible benefit.

## Decision Rules Applied

- **Absolute time vs system latency**: Both approaches (38–45µs) are negligible compared to the 50ms DB round-trip. This matches the skill's rule: "If the absolute time is negligible vs system latency — don't optimize, document why and move on."
- **Winner is < 10% faster in absolute system terms**: 7µs / 50,000µs = 0.014% end-to-end improvement. Not worth adopting the more complex approach.
- **Complexity cost**: The sort approach requires cloning the input, adding allocation and cognitive overhead for no practical gain.

## Recommendation

Keep the HashMap grouping. It's simpler (no clone needed), and the 7µs you'd save is noise against 50ms of I/O. If this endpoint ever becomes slow, the DB query is where to look — not the in-memory grouping.
