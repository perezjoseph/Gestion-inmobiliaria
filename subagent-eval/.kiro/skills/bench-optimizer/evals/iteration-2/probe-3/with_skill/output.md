# Optimization Analysis: Input Validation in Registration Endpoint

## Profile Summary

| Component | Time | % of Total |
|-----------|------|-----------|
| Password hashing (argon2) | 800ms | 94.1% |
| DB insert | 45ms | 5.3% |
| Input validation | 0.3ms | 0.035% |
| Response serialization | 0.1ms | 0.012% |
| **Total** | **850ms** | **100%** |

## Assessment

Input validation takes 0.3ms out of 850ms total — that's **0.035% of endpoint latency**. This is below the 1% threshold where optimization has any measurable impact on user experience.

Even if you made input validation infinitely fast (reduced it to 0ns), the endpoint would go from 850ms to 849.7ms. No user or monitoring system would detect that difference.

Additionally:
- This endpoint runs **~5 times per day**. It's a cold path, not a hot path.
- The real bottleneck is argon2 at 800ms (94.1%). That's by design — password hashing *should* be slow to resist brute-force attacks. Don't reduce it.
- The DB insert at 45ms is the only other meaningful contributor, and that's normal for a write with indexing.

Switching from string checks to compiled regex for input validation would add dependency complexity and code indirection for a gain that rounds to zero in the system context. The current string-check approach is likely more readable and equally correct.

## BENCH-VERDICT
status: SKIP
reason: ABSOLUTE_TIME_NEGLIGIBLE
bottleneck: password hashing (argon2) — 800ms / 94.1% of endpoint
time_budget: 850ms total endpoint latency
target_contribution: 0.3ms (0.035% of total)
