# Algorithm Optimization Report: `eval1_pago_generacion.rs`

## Summary

Applied algorithm design paradigms to reduce time complexity across all 4 functions in the payment generation module. Total improvements eliminate O(n²) and O(n log n) patterns, replacing them with O(n) alternatives using space-time tradeoffs and decrease-and-conquer techniques.

## Changes

### 1. `filtrar_existentes` — Space-Time Tradeoff (HashSet)

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(n × m) | O(n + m) |
| Space Complexity | O(1) | O(n) |
| Paradigm | Brute Force (nested `.any()`) | Space-Time Tradeoff |

**Rationale:** The original code used `.any()` inside `.filter()`, scanning all `pagos_existentes` for every element in `pagos_nuevos`. This is the classic "nested loop with inner search" anti-pattern. Building a `HashSet<(Uuid, NaiveDate)>` from existing payments provides O(1) amortized membership testing, reducing the overall complexity from O(n × m) to O(n + m).

**Tradeoff:** Allocates O(n) memory for the HashSet. Justified because payment collections in this domain can grow to thousands of records per month.

---

### 2. `ultimo_pago` — Decrease and Conquer (Iterator Max)

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(n log n) | O(n) |
| Space Complexity | O(n) (collected Vec) | O(1) |
| Paradigm | Sort-then-take-last | Decrease and Conquer (single-pass max) |

**Rationale:** The original collected filtered payments into a Vec, sorted the entire Vec by `fecha_pago`, then took `.last()`. Finding the maximum element only requires a single linear pass with `max_by_key`. This eliminates both the unnecessary sort and the intermediate Vec allocation.

**Tradeoff:** None — strictly better in both time and space.

---

### 3. `totales_por_mes` — Space-Time Tradeoff (HashMap Aggregation)

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(months × n) | O(n + m log m) where m = unique months |
| Space Complexity | O(months) | O(months) |
| Paradigm | Repeated full scans per month | Space-Time Tradeoff (single-pass aggregation) |

**Rationale:** The original code first collected unique months (O(n)), then for *each* unique month rescanned the entire `pagos` slice filtering by formatted date string comparison. This is O(months × n) — effectively O(n²) when months grow proportionally to n.

The optimized version performs a single pass over `pagos`, accumulating totals into a HashMap keyed by month string. The final sort of results by month key is O(m log m) where m is the number of unique months (typically 12-60), which is negligible.

Additionally, the original performed redundant `format("%Y-%m")` calls — once to collect unique months and again inside the per-month filter. The optimized version formats each date exactly once.

**Tradeoff:** HashMap allocation for month buckets. With typically ≤60 unique months, this is trivial.

---

### 4. `pendiente_por_contrato` — Eliminate Unnecessary Work (Direct Chain)

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(n) | O(n) |
| Space Complexity | O(k) where k = matching payments | O(1) |
| Paradigm | Collect-then-iterate | Direct iterator chain |

**Rationale:** The original collected filtered `monto` values into an intermediate `Vec<f64>` before summing. This allocates heap memory unnecessarily. Chaining `.sum()` directly on the iterator eliminates the allocation while maintaining identical semantics.

**Tradeoff:** None — strictly better in space with identical time complexity.

---

## Overall Impact

| Function | Before | After | Improvement |
|----------|--------|-------|-------------|
| `filtrar_existentes` | O(n × m) | O(n + m) | Quadratic → Linear |
| `ultimo_pago` | O(n log n) | O(n) | Linearithmic → Linear |
| `totales_por_mes` | O(months × n) | O(n) | Quadratic → Linear |
| `pendiente_por_contrato` | O(n) | O(n) | Eliminated unnecessary allocation |

## Paradigms Applied

1. **Space-Time Tradeoff** (×2) — HashSet for membership testing, HashMap for aggregation
2. **Decrease and Conquer** (×1) — Single-pass `max_by_key` instead of sort
3. **Eliminate Unnecessary Work** (×1) — Direct iterator chain without intermediate collection

## Correctness Notes

- `filtrar_existentes`: Semantically identical — same composite key `(contrato_id, fecha_vencimiento)` used for deduplication.
- `ultimo_pago`: Returns the same result — `max_by_key` on `fecha_pago` is equivalent to sorting and taking last.
- `totales_por_mes`: Output is sorted by month key, matching the original's `sort + dedup` ordering. Only "pagado" payments are summed, matching original filter logic.
- `pendiente_por_contrato`: Identical computation, just without the intermediate allocation.
