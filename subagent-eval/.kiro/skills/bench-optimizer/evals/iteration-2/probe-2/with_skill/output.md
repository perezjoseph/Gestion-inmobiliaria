# JSON Response Build — Benchmark Comparison

## Context

Actix handler building a 5-field JSON response (~1000 req/s). Comparing three user-proposed approaches plus two additional variants for completeness.

## Approaches Benchmarked

| # | Approach | Description |
|---|----------|-------------|
| 1 | `concat_fields` | `serde_json::to_string()` each field individually, concatenate into a String |
| 2 | `value_tree` | Build a `serde_json::Value` (via `json!` macro), then serialize the tree |
| 3 | `write_buffer` | Pre-allocated `String` buffer with `write!` macro (includes manual JSON escaping for `nombre`) |
| 3b | `write_buffer_no_escape` | Same as 3 but skipping escape logic (valid when inputs are known-safe: UUIDs, enum states, dates) |
| 4 | `struct_serialize` | Derive `Serialize` on a struct, call `serde_json::to_string` (idiomatic Actix pattern) |

## Measured Results

Benchmark ran with criterion (100 samples, release mode, realistic field values matching our domain).

| Approach | Mean Time | Relative to Winner |
|----------|-----------|-------------------|
| `write_buffer_no_escape` | **426 ns** | 1.00× (winner) |
| `write_buffer` (with escaping) | **710 ns** | 1.67× slower |
| `concat_fields` | **1,265 ns** | 2.97× slower |
| `struct_serialize` | **1,264 ns** | 2.97× slower |
| `value_tree` | **3,144 ns** | 7.38× slower |

## Analysis

1. **Winner: `write!` buffer** — The pre-allocated `String` + `write!` approach is the fastest by a significant margin. Without escape overhead (426 ns), it's ~3× faster than serde-based approaches and ~7.4× faster than the Value tree. Even with manual escaping for the `nombre` field (710 ns), it still beats all serde approaches.

2. **`concat_fields` ≈ `struct_serialize`** — These are statistically identical (~1,265 ns). The per-field `to_string` approach doesn't save anything over just deriving Serialize on a struct. Use the struct approach — it's safer and idiomatic.

3. **`value_tree` is the slowest** — At 3,144 ns it's 7.4× slower than the winner. Building a `serde_json::Value` tree allocates for every node (BTreeMap, String keys, Value variants) and then serializes the whole thing. Double work.

## Practical Impact at 1000 req/s

| Approach | Time per request | Total CPU/s at 1000 rps |
|----------|-----------------|------------------------|
| `write_buffer_no_escape` | 426 ns | 0.426 ms (0.04% of 1s) |
| `struct_serialize` | 1,264 ns | 1.264 ms (0.13% of 1s) |
| `value_tree` | 3,144 ns | 3.144 ms (0.31% of 1s) |

At 1000 req/s, even the slowest approach only consumes 0.3% of a CPU-second. The difference between winner and loser is ~2.7 ms of CPU time per second. This is negligible compared to DB queries, network I/O, and TLS overhead in this handler.

## Recommendation

The `write!` buffer approach IS measurably fastest, but:

- It sacrifices type safety (no compile-time field validation)
- It requires manual JSON escaping (security risk if forgotten)
- The absolute savings at 1000 rps are ~2.7 ms/s of CPU time — irrelevant

**Use `struct_serialize` (approach 4).** It's the idiomatic Actix pattern, type-safe, handles escaping correctly via serde, and at 1.26 µs per call it contributes < 0.13% of your total CPU budget. The 3× speedup of `write!` isn't worth the maintenance and correctness risk.

If you scale to 100k+ rps or profile shows JSON serialization as a bottleneck, revisit with `write!` buffer.

## Benchmark File

Kept at `backend/benches/json_response_build.rs` for regression detection.

Run with:
```bash
cargo bench --bench json_response_build
```

## BENCH-VERDICT
status: MEASURED
winner: write_buffer_no_escape
winner_time: 426ns
runner_up_time: 710ns (write_buffer with escaping), 1264ns (struct_serialize), 3144ns (value_tree)
speedup: 197% vs struct_serialize, 638% vs value_tree
recommendation: KEEP_CURRENT
reason: Absolute time is negligible at 1000 rps (0.13% CPU); struct_serialize is safer and idiomatic. Winner sacrifices type safety and correct escaping for 800ns savings per request.
