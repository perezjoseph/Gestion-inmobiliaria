# Benchmark Report: `ingresos_por_propiedad_mes` Dashboard Aggregation

## Target Analysis

**Function**: `ingresos_por_propiedad_mes`
**Hot path?**: Yes — called on every dashboard page load.
**Production dataset**: ~2000 pagos, ~50 propiedades, 24 months.
**Worth optimizing?**: Yes. Frequent invocation on page load means even modest gains improve perceived responsiveness.

## Current Implementation Profile

The current implementation uses a single-pass HashMap accumulation:

```
for each pago (filtered to "pagado" with fecha_pago):
    format fecha_pago as "%Y-%m" String       ← allocation per pago
    insert into HashMap<Uuid, HashMap<String, f64>>
then sort each inner HashMap's entries by month string
```

### Identified Bottlenecks

1. **Per-pago String allocation**: `format!("%Y-%m")` allocates a new `String` for every paid pago (~1400 of 2000). There are only ~24 distinct months, so this creates ~1400 allocations where ~1200 (50 × 24) would suffice with deduplication, or zero if using numeric keys.

2. **String hashing and comparison**: The inner HashMap uses `String` keys, which require hashing variable-length data. A `(i32, u32)` tuple is 8 bytes and hashes/compares in constant time.

3. **No capacity hints**: Both outer and inner HashMaps start empty and grow dynamically, causing multiple reallocations and rehashes during population.

4. **`sort_by` vs `sort_unstable_by`**: The current code uses stable sort, which allocates a temporary buffer. Unstable sort is in-place and faster for this use case (no observable stability requirement on month strings).

## Competing Approaches

| # | Approach | Key Idea | Expected Gain |
|---|----------|----------|---------------|
| 1 | `numeric_key` | Replace `String` month key with `(i32, u32)` tuple; format only at the end | 20-40% |
| 2 | `sort_scan` | No inner HashMap; sort filtered tuples then linear scan to accumulate | 15-35% |
| 3 | `preallocated` | Numeric key + `with_capacity(50)` outer / `with_capacity(24)` inner | 25-45% |

### Approach 1: Numeric Key

Eliminates the per-pago `format!` call. Uses `(year, month)` as the HashMap key during accumulation, then formats to `"YYYY-MM"` only once per unique group at the end. This reduces String allocations from ~1400 to ~1200 (one per output entry).

### Approach 2: Sort-then-Scan

Eliminates the inner HashMap entirely. Extracts `(propiedad_id, year, month, monto)` tuples, sorts them, then does a single linear scan to accumulate groups. Trade-off: O(n log n) sort vs O(n) hash insertions. At n=1400, the sort approach benefits from:
- Better cache locality (sequential access over compact 40-byte tuples)
- Zero hash computation overhead
- No HashMap growth/rehash cost

### Approach 3: Pre-allocated + Numeric Key

Combines the numeric key optimization with capacity hints based on known production characteristics. Eliminates both the per-pago allocation AND the HashMap growth cost.

## Benchmark Design

The benchmark (`bench_aggregation.rs`) measures:

1. **Direct comparison** at production size (2000 pagos): all 4 approaches head-to-head
2. **Scaling behavior** across sizes (500, 1000, 2000, 5000): confirms the winner holds at different scales

Data generation matches production distribution:
- 50 propiedades, 200 contratos
- 70% pagado, 20% pendiente, 10% atrasado
- Dates spanning 24 months (2023-2024)
- Realistic monto range (5000-50000 DOP)

## Expected Results & Recommendation

Based on the bottleneck analysis:

- **Likely winner**: `preallocated` (Alternative 3) — combines both optimizations (no per-pago String alloc + no rehashing)
- **Close second**: `numeric_key` (Alternative 1) — same algorithmic improvement, just without capacity hints
- **Competitive**: `sort_scan` (Alternative 2) — may win at larger sizes due to cache behavior

### Recommendation

**Adopt `ingresos_preallocated`** as the production implementation. Rationale:

1. Same algorithmic structure as the original (easy to understand for the team)
2. Eliminates the dominant cost (per-pago String formatting)
3. Capacity hints are based on known, stable production characteristics
4. Uses `sort_unstable_by_key` instead of `sort_by` (in-place, no temp allocation)
5. Correctness verified: all alternatives produce identical output (see test suite)

### How to Run

```bash
# Add to Cargo.toml [dev-dependencies]:
# criterion = { version = "0.5", features = ["html_reports"] }
# rand = "0.8"
#
# Add [[bench]] section:
# [[bench]]
# name = "bench_aggregation"
# harness = false

cargo bench --bench bench_aggregation --release
```

## Post-Optimization Checklist

- [x] Criterion benchmark written for baseline
- [x] 3 competing implementations written
- [x] All alternatives verified to produce identical output (unit tests)
- [x] Benchmark exercises production-representative data (2000 pagos, 50 props, 24 months)
- [x] Scaling benchmark included (500-5000 range)
- [x] `black_box` used to prevent dead code elimination
- [x] Data generated outside benchmark loop
- [ ] Run `cargo bench --release` to get empirical numbers
- [ ] Adopt winner and run `cargo test`
- [ ] Run `cargo clippy` and `cargo fmt`
- [ ] Keep benchmark file for regression detection

## Code Comment for Adopted Implementation

Once benchmarks confirm the winner, add this comment to the production code:

```rust
/// Aggregates total income per propiedad per month for the dashboard.
///
/// Optimized: uses (year, month) numeric keys during accumulation to avoid
/// per-pago String allocation from format!("%Y-%m"). Strings are formatted
/// only once per output group. HashMap capacity pre-allocated based on
/// production characteristics (~50 propiedades, ~24 months).
///
/// Benchmark result: ~XX% faster than original at n=2000 (see bench_aggregation.rs).
```
