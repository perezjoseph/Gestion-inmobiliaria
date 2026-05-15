# Algorithm Optimization Report

## Summary

Analyzed 5 functions for algorithm efficiency. Applied optimizations to 3 functions with genuine complexity problems. Left 2 functions unchanged where BTreeMap usage is intentional and the algorithms are already optimal.

## Function-by-Function Analysis

### 1. `reporte_envejecimiento` — NO CHANGE

| Aspect | Value |
|--------|-------|
| Complexity | O(n) — single pass |
| Data Structure | BTreeMap (4 fixed buckets) |
| Decision | **Keep as-is** |

**Reasoning:** The BTreeMap holds only 4 entries (the aging buckets) and ensures they appear in sorted order ("01-30 días" < "31-60 días" < ...) for the report output. With a fixed number of keys, BTreeMap vs HashMap is irrelevant to performance — the O(log k) insertion where k=4 is effectively O(1). The single-pass iteration over ~500 payments is already optimal. Replacing BTreeMap with HashMap would break the sorted output requirement.

---

### 2. `inquilinos_sin_atrasos` — OPTIMIZED

| Aspect | Before | After |
|--------|--------|-------|
| Complexity | O(n × m × p) | O(n + m + p) |
| Paradigm | Brute Force (triple nested loop) | Space-Time Tradeoff |
| Worst case | ~40,000,000 comparisons | ~2,300 operations |

**Problem:** Triple nested loop: for each inquilino (100), scan all contratos (200), and for each matching contrato, scan all pagos (2000). Even with early breaks, worst case is O(100 × 200 × 2000).

**Solution:** Precompute two lookup structures:
1. `HashSet<Uuid>` of contrato IDs that have at least one "atrasado" payment — built in O(p)
2. `HashMap<Uuid, Vec<Uuid>>` mapping inquilino_id → their contrato IDs — built in O(m)

Then for each inquilino, check if any of their contrato IDs appear in the late set. This is O(n × avg_contratos_per_inquilino) ≈ O(n × 2) for the final check.

**Tradeoff:** Uses O(m + p) additional memory for the lookup structures. With 200 contratos and 2000 pagos, this is negligible.

---

### 3. `tendencia_pagos_acumulados` — OPTIMIZED

| Aspect | Before | After |
|--------|--------|-------|
| Complexity | O(months × n) | O(n + m log m) |
| Paradigm | Repeated prefix sum | Transform and Conquer + Space-Time Tradeoff |
| Operations | ~48,000 with repeated string formatting | ~2,120 |

**Problem:** For each of 24 months, the code re-scans all 2000 payments and re-formats dates to strings for comparison. This is O(24 × 2000) = ~48,000 iterations, each involving string allocation and comparison.

**Solution:** 
1. Single pass: group payment amounts into a `BTreeMap<String, f64>` by month — O(n)
2. BTreeMap gives chronological order for free (YYYY-MM strings sort correctly)
3. Compute running cumulative sum over the sorted months — O(m)

**Why BTreeMap here:** The original used sort+dedup on a Vec. BTreeMap achieves the same sorted-unique-months result while also accumulating totals in the same pass. It replaces both the dedup step and the per-month re-scan.

**Tradeoff:** O(m) additional memory for the monthly totals map. With 24 months, this is trivial.

---

### 4. `detectar_pagos_duplicados` — OPTIMIZED

| Aspect | Before | After |
|--------|--------|-------|
| Complexity | O(n²) | O(n) average |
| Paradigm | Brute Force (pairwise) | Space-Time Tradeoff (HashMap grouping) |
| Operations | ~2,000,000 comparisons | ~2,000 insertions + tiny pairwise within groups |

**Problem:** O(n²) pairwise comparison of all 2000 payments. Each pair checks 5 conditions. Total: ~2,000,000 comparisons.

**Solution:** Group payments by composite key `(contrato_id, fecha_vencimiento, monto.to_bits())` into a HashMap. Only payments in the same bucket can be duplicates. Since duplicates are < 1%, most buckets have exactly 1 entry. Pairwise comparison only happens within the rare multi-entry buckets.

**Key design choice:** Using `f64::to_bits()` for the monto component ensures exact floating-point equality matching (same behavior as `==` but usable as a HashMap key). This preserves the original semantics where two payments with the exact same monto value are considered potential duplicates.

**Tradeoff:** O(n) additional memory for the HashMap. With 2000 payments, this is ~2000 entries × ~40 bytes ≈ 80KB.

---

### 5. `resumen_metodos_pago` — NO CHANGE

| Aspect | Value |
|--------|-------|
| Complexity | O(n) — single pass |
| Data Structure | BTreeMap<String, HashMap<String, f64>> |
| Decision | **Keep as-is** |

**Reasoning:** Already optimal. Single pass over payments, O(1) amortized insertion into nested maps. The outer BTreeMap ensures months appear in chronological order for the stacked bar chart (required by the UI). The inner HashMap is correct because payment method ordering doesn't matter — only the totals per method are needed. No optimization possible.

---

## Paradigms Applied

| Function | Paradigm | Justification |
|----------|----------|---------------|
| `inquilinos_sin_atrasos` | Space-Time Tradeoff | Precomputed HashSet/HashMap eliminates nested scans |
| `tendencia_pagos_acumulados` | Transform and Conquer + Space-Time Tradeoff | Restructure into grouped totals, then single cumulative pass |
| `detectar_pagos_duplicados` | Space-Time Tradeoff | HashMap grouping by composite key reduces to bucket-local comparisons |

## Key Decisions Explained

### Why BTreeMap was kept in functions 1 and 5
Both functions produce output for UI rendering that requires chronological/sorted ordering. BTreeMap provides this ordering guarantee with negligible overhead (O(log k) where k is small). Replacing with HashMap would require a separate sort step and make the code less clear about its ordering intent.

### Why `inquilinos_sin_atrasos` needed optimization despite small n
While 100 inquilinos is small, the triple nesting with 200 contratos and 2000 pagos creates up to 40M comparisons in the worst case. The optimized version is both faster AND more readable — the intent (which contratos have late payments?) is clearer with explicit data structures.

### Why `tendencia_pagos_acumulados` uses BTreeMap instead of sort+dedup
The original collected months into a Vec, sorted, deduped, then re-scanned payments for each month. Using BTreeMap combines collection, deduplication, and sorting into one structure while also accumulating per-month totals — eliminating the need for the expensive per-month re-scan entirely.
