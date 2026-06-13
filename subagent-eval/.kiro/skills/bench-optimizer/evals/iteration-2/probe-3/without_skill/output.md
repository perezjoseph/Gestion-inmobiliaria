# Response: Optimizing Input Validation in Registration Endpoint

## Don't Optimize This

Looking at your profile data, the input validation takes **0.3ms** out of an **850ms** total request. That's 0.035% of your endpoint's latency. Even if you made validation instantaneous (0ms), you'd save 0.3ms — completely imperceptible to the user.

Meanwhile, password hashing dominates at 800ms (94% of total time).

With ~5 registrations per day, this endpoint processes roughly 5 requests daily. Even under the most generous interpretation, optimizing 0.3ms × 5 calls = 1.5ms saved per day. That's not a meaningful improvement by any measure.

## Where to Actually Look (If You Want to Improve This)

If the 850ms total bothers you, the only lever that matters is the argon2 hashing at 800ms. A few options:

1. **Offload hashing to a background task** — return a 202 Accepted immediately, hash the password async, and activate the account once complete. The user gets a fast response and the registration completes momentarily.

2. **Tune argon2 parameters** — if your current params are more aggressive than needed for your threat model (5 registrations/day on an internal property management platform), you could reduce memory/iterations slightly. But be careful here — don't weaken security for speed on an endpoint that barely gets traffic.

3. **Move hashing off the request thread** — use `actix_web::web::block` or `tokio::task::spawn_blocking` to ensure the 800ms hash doesn't block your async runtime (you're likely already doing this, but worth confirming).

## The Compiled Regex Idea

To directly answer your question: yes, a compiled regex (via `lazy_static!` or `std::sync::LazyLock`) is marginally faster than multiple string checks on repeated calls. But "marginally faster" here means going from ~0.3ms to maybe ~0.1ms. The difference is 0.2ms on an endpoint called 5 times per day.

This is a textbook case of optimizing the wrong thing. The effort of writing, testing, and maintaining a compiled regex for registration validation would cost more developer time than it could ever save in compute time over the lifetime of the application.

## Summary

| Component | Time | % of Total | Worth Optimizing? |
|-----------|------|-----------|-------------------|
| Password hashing | 800ms | 94% | Only if UX is a concern |
| DB insert | 45ms | 5.3% | No (already fast) |
| Input validation | 0.3ms | 0.035% | No |
| Serialization | 0.1ms | 0.012% | No |

Keep your multiple string checks. They're clear, readable, and fast enough. Spend your time on features that matter to your users.
