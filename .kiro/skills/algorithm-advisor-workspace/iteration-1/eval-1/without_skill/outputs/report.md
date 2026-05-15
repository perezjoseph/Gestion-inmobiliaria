# Algorithm Optimization Report — `eval1_pago_generacion.rs`

## Summary

Four functions were optimized, reducing overall time complexity from quadratic/superlinear to linear in all cases.

---

## Function-by-Function Analysis

### 1. `filtrar_existentes`

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n × m) | O(n + m) |
| Paradigm | Nested iteration | Hash-based lookup |

**Problem:** For each element in `pagos_nuevos` (m items), the code scanned all `pagos_existentes` (n items) using `.any()`, resulting in O(n × m) comparisons.

**Fix:** Pre-build a `HashSet<(Uuid, NaiveDate)>` from `pagos_existentes` in O(n), then check membership for each new pago in O(1) amortized. Total: O(n + m).

---

### 2. `ultimo_pago`

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n log n) | O(n) |
| Paradigm | Sort-then-take-last | Single-pass max |

**Problem:** The code collected filtered payments into a Vec, sorted the entire Vec by `fecha_pago`, then took the last element. Sorting is O(n log n) when only the maximum is needed.

**Fix:** Replace sort + last with `Iterator::max_by_key`, which finds the maximum in a single O(n) pass with no allocation.

---

### 3. `totales_por_mes`

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(months × n) | O(n + k log k) where k = unique months |
| Paradigm | Repeated full scans | Single-pass accumulation (HashMap) |

**Problem:** The code first collected unique months (O(n log n) sort + dedup), then for each unique month rescanned the entire `pagos` slice, re-formatting dates and comparing strings. This is O(months × n) — effectively quadratic when months grow with data.

**Fix:** Single O(n) pass accumulating totals into a `HashMap<String, f64>`, filtering for `"pagado"` inline. Then sort the k resulting entries (O(k log k), where k ≪ n). Also eliminates redundant `format!` calls per comparison.

---

### 4. `pendiente_por_contrato`

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n) (same) | O(n) (same) |
| Improvement | Eliminates unnecessary heap allocation | Direct iterator sum |

**Problem:** The code collected filtered `monto` values into an intermediate `Vec<f64>` before summing. While still O(n), this allocates heap memory unnecessarily.

**Fix:** Chain `.filter().map().sum()` directly on the iterator, avoiding the intermediate `Vec` allocation entirely. Same time complexity, better space complexity (O(1) auxiliary vs O(k) where k = matching payments).

---

## Design Paradigms Applied

1. **Space-time tradeoff (Hash lookup):** Trading O(n) memory for O(1) lookup in `filtrar_existentes`.
2. **Decrease and conquer (Single-pass max):** Reducing the "find last" problem to a single linear scan in `ultimo_pago`.
3. **Transform and conquer (Accumulator pattern):** Transforming the grouping problem into a single-pass HashMap accumulation in `totales_por_mes`.
4. **Iterator fusion (Allocation elimination):** Fusing filter/map/sum into a single iterator pipeline in `pendiente_por_contrato`.

## Style Compliance

- Uses `&[T]` over `&Vec<T>` ✓
- Iterator chains over loops ✓
- No unnecessary `.clone()` ✓
- No `unwrap()`/`expect()` ✓
- Idiomatic Rust 2024 patterns ✓
