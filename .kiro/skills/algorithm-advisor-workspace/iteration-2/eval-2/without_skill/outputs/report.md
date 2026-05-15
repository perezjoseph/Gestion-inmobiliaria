# Algorithm Optimization Report

## Summary

| Function | Original Complexity | Optimized Complexity | Action |
|----------|-------------------|---------------------|--------|
| `reporte_envejecimiento` | O(n) | O(n) | **No change** — already optimal |
| `inquilinos_sin_atrasos` | O(I × C × P) | O(I + C + P) | **Optimized** — index-based lookups |
| `tendencia_pagos_acumulados` | O(M × N) | O(N + M log M) | **Optimized** — group-then-accumulate |
| `detectar_pagos_duplicados` | O(N²) | O(N) amortized | **Optimized** — hash-based grouping |
| `resumen_metodos_pago` | O(N log M) | O(N log M) | **No change** — already optimal |

---

## Detailed Analysis

### 1. `reporte_envejecimiento` — NO CHANGE

**Reasoning:** The BTreeMap here holds at most 4 entries (the 4 aging buckets). The cost difference between BTreeMap and HashMap for 4 keys is negligible — a few nanoseconds at most. The BTreeMap is intentional because the report requires buckets in sorted ascending order. Replacing it with a HashMap would require a separate sort step, adding complexity for zero real-world gain.

The function already does a single O(n) pass over payments. This is optimal.

**Decision:** Leave as-is. The BTreeMap is the correct choice.

---

### 2. `inquilinos_sin_atrasos` — OPTIMIZED

**Original problem:** Triple-nested loop — for each inquilino, scan all contratos, and for each matching contrato, scan all pagos. Worst case: O(100 × 200 × 2000) = O(40,000,000) comparisons.

**Optimization approach:** Pre-build two indexes in linear time:
1. **Set of contrato_ids with late payments** — single pass over pagos, O(P). HashSet gives O(1) membership test.
2. **Map of inquilino_id → their contrato_ids** — single pass over contratos, O(C).

Then for each inquilino, check if any of their contratos appear in the late set. With ~100 inquilinos averaging ~2 contratos each, the final pass is O(I × avg_contratos_per_inquilino) ≈ O(200).

**Total:** O(P + C + I) ≈ O(2000 + 200 + 100) = O(2300) vs. original O(40M) worst case.

**Why not just a single HashSet of inquilino_ids?** We could join pagos→contratos→inquilinos in one pass, but that would require a contrato_id→inquilino_id map anyway. The two-index approach is clearer and equally efficient.

---

### 3. `tendencia_pagos_acumulados` — OPTIMIZED

**Original problem:** For each of M months, the code re-scans ALL N payments to compute the cumulative sum up to that month. This is O(M × N). With 24 months and 2000 payments, that's ~48,000 iterations with repeated string formatting on every comparison.

**Optimization approach:**
1. **Single pass** over payments to group totals by month into a HashMap — O(N).
2. **Sort** the month keys — O(M log M), trivial for M=24.
3. **Single pass** over sorted months computing a running cumulative sum — O(M).

**Total:** O(N + M log M) ≈ O(2000) vs. original O(48,000) with expensive string operations.

**Key insight:** A HashMap alone doesn't solve this — the cumulative aspect requires sorted iteration. The fix is to separate concerns: aggregate first (HashMap), sort second, accumulate third. This avoids the repeated full-scan prefix sum.

---

### 4. `detectar_pagos_duplicados` — OPTIMIZED

**Original problem:** O(N²) pairwise comparison of all payments. With N=2000, that's ~2,000,000 comparisons.

**Optimization approach:** Group payments by their deduplication key `(contrato_id, fecha_vencimiento, monto)` into a HashMap. Payments within the same bucket are duplicates of each other. Since duplicates are rare (< 1%), most buckets have exactly 1 entry and are skipped.

**Composite key:** Uses `(Uuid, NaiveDate, u64)` where the u64 is `f64::to_bits()` for exact floating-point comparison (matching the original `==` semantics).

**Total:** O(N) for the grouping pass. The inner pairwise loop within buckets is O(k²) per bucket, but since duplicates are < 1% and typically come in pairs (k=2), this is effectively O(1) per bucket.

**Why `to_bits()` instead of an epsilon comparison?** The original code uses `==` on f64, so we preserve that exact semantics. In this domain, payment amounts are stored values (not computed), so exact equality is appropriate.

---

### 5. `resumen_metodos_pago` — NO CHANGE

**Reasoning:** This function already does a single O(N) pass over payments, aggregating into a nested BTreeMap<String, HashMap<String, f64>>. The BTreeMap is intentional — months must be in chronological order for the stacked bar chart. The inner HashMap is correct because payment methods don't need ordering.

With ~2000 payments, 24 months, and 4 payment methods, this is already optimal. The BTreeMap insertion cost is O(log 24) ≈ O(5) per payment — negligible.

**Decision:** Leave as-is. Already the right data structure and algorithm.

---

## Design Decisions

1. **Preserved BTreeMap where ordering is required** — both `reporte_envejecimiento` and `resumen_metodos_pago` need sorted output. Replacing with HashMap would just move the sort cost elsewhere.

2. **Used HashSet for membership testing** — in `inquilinos_sin_atrasos`, checking "does this contrato have late payments?" is a classic set membership problem.

3. **Used HashMap for grouping** — in both `tendencia_pagos_acumulados` and `detectar_pagos_duplicados`, the core operation is "group by key then process groups."

4. **Preserved exact f64 semantics** — used `to_bits()` rather than introducing epsilon comparisons, since payment amounts are stored values with exact representations.

5. **Used `with_capacity` where size is predictable** — for the inquilino→contratos map, we know the approximate number of inquilinos upfront.
