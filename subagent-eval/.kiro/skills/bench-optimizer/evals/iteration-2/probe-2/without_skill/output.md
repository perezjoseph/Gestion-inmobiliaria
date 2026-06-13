# Comparison: JSON Response Building Approaches in Actix Handler

## Context

- Rust / Actix-web handler
- ~1000 req/s throughput
- Each response has 5 fields (typical small DTO)

## The Three Approaches

### 1. `serde_json::to_string()` per field + concatenation

Serialize each field individually, then concatenate strings with manual JSON punctuation.

```rust
let mut body = String::from("{");
body.push_str(&format!("\"id\":{},", serde_json::to_string(&id)?));
body.push_str(&format!("\"name\":{},", serde_json::to_string(&name)?));
// ... 5 fields
body.push('}');
```

### 2. Build a `serde_json::Value` tree then serialize

Construct a `serde_json::Value::Object` and serialize it once.

```rust
let value = serde_json::json!({
    "id": id,
    "name": name,
    "status": status,
    "amount": amount,
    "created_at": created_at,
});
let body = serde_json::to_string(&value)?;
```

### 3. Pre-allocated `String` buffer with `write!`

Write JSON directly into a pre-sized buffer using format macros.

```rust
use std::fmt::Write;
let mut buf = String::with_capacity(256);
write!(buf, r#"{{"id":{},"name":"{}","status":"{}","amount":{},"created_at":"{}"}}"#,
    id, name, status, amount, created_at)?;
```

## Expected Performance (5-field response, ~200–300 byte output)

| Approach | Estimated Latency per call | Relative Speed | Key Overhead |
|----------|---------------------------|----------------|--------------|
| 1. Per-field serialize + concat | ~1,800–2,200 ns | **Slowest** (1.0×) | 5 separate serde serializations, 5 allocations, string concatenation |
| 2. `serde_json::Value` tree | ~800–1,100 ns | **Middle** (~2× faster than #1) | Heap-allocated `Value` variants (Map, String wrappers), one serialization pass |
| 3. Pre-allocated `String` + `write!` | ~150–300 ns | **Fastest** (~6–8× faster than #1, ~3–4× faster than #2) | No serde overhead, no intermediate allocations, single buffer write |

## Winner: Approach 3 — Pre-allocated `String` buffer with `write!`

### Specific Numbers (estimated for a 5-field response)

- **3× to 4× faster than approach 2** (Value tree)
- **6× to 8× faster than approach 1** (per-field serialize + concat)
- Absolute time: ~150–300 ns vs ~800–1,100 ns vs ~1,800–2,200 ns

### Why It Wins

1. **Zero serde overhead**: No schema traversal, no `Serialize` trait dispatch, no intermediate `Value` heap objects.
2. **Single allocation**: One `String::with_capacity(256)` is enough for a 5-field response. Approaches 1 and 2 allocate multiple intermediate strings/objects.
3. **No type erasure**: `serde_json::Value` boxes every value into an enum variant with heap-allocated strings. `write!` goes directly to the output buffer.
4. **CPU-cache friendly**: Sequential writes into a contiguous buffer vs. pointer-chasing through a `BTreeMap`/`Map<String, Value>`.

### Why You Might NOT Use Approach 3

- **Maintenance burden**: Manual formatting is error-prone. If a field is a user-provided string, you must handle JSON escaping yourself (or call `serde_json::to_string` just for that field).
- **No automatic escaping**: `write!` won't escape `"` or `\` in string values. You'd need a helper or use a crate like `itoa`/`ryu` for numbers and manual escaping for strings.
- **Schema drift**: Adding/removing fields requires manual buffer updates with no compile-time safety.

## Practical Recommendation

For a handler at 1,000 req/s with 5 fields, **the performance difference is negligible in absolute terms** (~1–2 µs saved per request = ~1–2 ms/s total CPU). The idiomatic and safest approach is:

```rust
#[derive(serde::Serialize)]
struct Response {
    id: i64,
    name: String,
    status: String,
    amount: f64,
    created_at: String,
}

HttpResponse::Ok().json(Response { ... })
```

This uses `serde_json::to_string` on a typed struct (not `Value`), which runs at approximately **300–500 ns** for 5 fields — nearly as fast as the manual `write!` approach but with full type safety, automatic escaping, and compile-time correctness.

**The real winner for production code is `#[derive(Serialize)]` on a struct**, which gives you ~90% of approach 3's speed with none of the maintenance cost. Reserve the `write!` approach for hot paths doing >100K req/s where every nanosecond matters.

## Caveat

These numbers are estimates based on published benchmarks of `serde_json` internals and general Rust formatting performance. For authoritative results in your specific workload, you should run criterion benchmarks against your actual response shape and data. The relative ordering (3 > struct-derive > Value > per-field concat) is well-established, but absolute numbers vary with CPU, allocator, and data size.
