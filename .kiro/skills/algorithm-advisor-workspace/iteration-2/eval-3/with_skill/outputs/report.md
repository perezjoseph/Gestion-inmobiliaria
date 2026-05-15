# Algorithm Optimization Report: eval3_paradigm_selection.rs

## Summary

This file contains four functions for a property management platform. Each requires a **different** optimization strategy — and critically, two of them are already optimal and should NOT be changed. Recognizing when to leave code alone is as important as knowing when to optimize.

| Function | Paradigm | Before | After | Changed? |
|----------|----------|--------|--------|----------|
| `aplicar_pago_parcial` | Greedy | O(n log n) | O(n log n) | No — already optimal |
| `rentabilidad_por_propiedad_mes` | Space-Time Tradeoff | O(P × M × (pagos + gastos)) | O(pagos + gastos + P×M) | Yes |
| `mejor_mes_mantenimiento` | N/A (already O(n)) | O(12 × n) = O(n) | O(n) | No — already optimal |
| `cascada_morosidad` | Dynamic Programming (prefix sum) | O(months × pagos) | O(pagos + months) | Yes |

---

## Function 1: `aplicar_pago_parcial`

### Paradigm: Greedy

### Analysis

The original code sorts pending payments by `fecha_vencimiento` (oldest first) then greedily allocates the available amount. This is textbook Greedy:

- **Greedy choice property**: Paying the oldest debt first minimizes accumulated late penalties. An exchange argument proves this: if you allocate to a newer debt while an older one remains, swapping the allocation reduces total penalty days.
- **Optimal substructure**: After allocating to the oldest debt, the remaining problem (allocate remainder to remaining debts) has the same structure.

### Why NOT a min-heap?

The comment in the code asks whether a min-heap (O(n) construction) would be better than sort (O(n log n)). The answer is **no**:

1. **n = 1-12 elements** — at this scale, sort vs heap is irrelevant. The constant factors of heap operations (pointer chasing, cache misses) likely make it slower.
2. **After construction, you extract ALL elements in order** — heap extraction is O(n log n) total, same as sort.
3. **Sort produces a contiguous sorted slice** — better cache locality for the subsequent greedy pass.
4. **Readability** — `sort_by_key` is immediately clear; a BinaryHeap adds cognitive overhead for zero benefit.

### Verdict: No change needed.

---

## Function 2: `rentabilidad_por_propiedad_mes`

### Paradigm: Space-Time Tradeoff (precomputed lookup tables)

### Analysis

The original has three nested dimensions: for each propiedad, for each month, scan all pagos (with an inner scan of contratos to resolve propiedad) and all gastos. This is effectively:

```
O(P × M × (pagos × contratos + gastos))
```

With P=50, M=24, pagos=2000, contratos=~200, gastos=500, that's roughly **50 × 24 × (2000×200 + 500) = 480 million** operations in the worst case.

### Optimization Strategy

Build three lookup structures in single passes:

1. **`contrato_to_propiedad: HashMap<Uuid, Uuid>`** — resolves pago → propiedad in O(1) instead of scanning contratos
2. **`ingresos_map: HashMap<(Uuid, String), f64>`** — pre-aggregates income by (propiedad, month)
3. **`egresos_map: HashMap<(Uuid, String), f64>`** — pre-aggregates expenses by (propiedad, month)

Then assembly is O(P × M) with O(1) lookups.

### Complexity Reduction

- **Before**: O(P × M × (pagos × contratos + gastos)) ≈ O(n³) behavior
- **After**: O(pagos + gastos + contratos + P × M) ≈ O(n) in total input size

### Why Space-Time Tradeoff and not other paradigms?

- **Not Divide & Conquer**: The subproblems (different propiedades) share the same pagos/gastos data — they're not independent.
- **Not DP**: There's no overlapping subproblem structure — each (propiedad, month) cell is computed independently.
- **Not Transform & Conquer (sort)**: Sorting doesn't help because we need random access by composite key (propiedad + month), not sequential access.
- **Space-Time Tradeoff is the exact fit**: We trade O(pagos + gastos) memory for HashMaps that eliminate repeated linear scans.

---

## Function 3: `mejor_mes_mantenimiento`

### Paradigm: None needed — already O(n)

### Analysis

The outer loop runs exactly 12 times (fixed constant). The inner loop scans ~200 contracts. Total work: **12 × 200 = 2,400 comparisons**. This is O(n) where n = number of contracts.

### Why NOT an interval tree?

- **Construction cost**: O(n log n) to build the tree for 200 contracts.
- **Query cost**: O(log n + k) per query where k = number of overlapping intervals.
- **Total for 12 queries**: O(n log n) + O(12 × (log n + k)).
- **Comparison**: The brute force does 2,400 simple comparisons. The interval tree does ~200 × 8 = 1,600 operations just for construction, plus query overhead, plus the complexity of maintaining the tree structure.
- **Verdict**: The interval tree has **higher constant factors** and **more complex code** for the same or worse practical performance at n=200.

### Why NOT a sweep line?

- **Sort**: O(n log n) ≈ 200 × 8 = 1,600 operations.
- **Sweep**: O(n) = 200 operations.
- **Total**: ~1,800 operations — comparable to brute force's 2,400, but with significantly more complex code (event points, active set tracking).
- **Verdict**: Not worth the complexity for a negligible constant-factor improvement.

### Verdict: No change needed. The simplest correct solution is already efficient enough.

---

## Function 4: `cascada_morosidad`

### Paradigm: Dynamic Programming (prefix sum / cumulative computation)

### Analysis

The original computes, for each month M, the count of payments from **previous** months that are still unpaid. It does this by rescanning ALL payments for each month:

```
O(months × pagos) = O(24 × 2000) = 48,000 operations
```

The key insight is that this is a **cumulative** quantity with **optimal substructure**:

```
cascade(month_M) = cascade(month_{M-1}) + unpaid_payments_due_in_month_{M-1}
```

This is the recurrence relation that makes it a DP problem.

### Optimization Strategy

1. **Group**: Count unpaid payments per vencimiento month — single pass O(pagos)
2. **Prefix sum**: Accumulate counts across months — O(months)
3. **Result**: For month M, the cascade value is the prefix sum up to (but not including) M

### Complexity Reduction

- **Before**: O(months × pagos) with repeated string formatting per comparison
- **After**: O(pagos + months) — each payment examined once, each month accumulated once

### Why DP (prefix sum) and not other paradigms?

- **Not Space-Time Tradeoff (HashMap)**: A HashMap alone doesn't capture the cumulative "from all previous months" semantics. You'd still need to sum across multiple keys.
- **Not Greedy**: There's no optimization choice being made — this is a reporting/counting problem.
- **Not Transform & Conquer (sort)**: Sorting payments by date would allow a sweep, but the prefix sum approach is simpler and achieves the same O(n) result without modifying the input.
- **DP (prefix sum) is the exact fit**: The cumulative count has optimal substructure (each month's value builds on the previous), and computing it bottom-up avoids redundant work.

---

## Key Takeaways

1. **Not every function needs optimization.** Two of four functions were already optimal. Applying complex data structures (heaps, interval trees) to already-efficient code is a common anti-pattern.

2. **Dataset size matters.** For n ≤ 12 (pagos_pendientes) or a fixed 12-iteration loop (maintenance), asymptotic improvements are meaningless — constant factors dominate.

3. **Each problem has a natural paradigm.** Forcing HashMap lookups onto a cumulative counting problem (cascada_morosidad) would be awkward and incomplete. The prefix sum / DP approach matches the problem's mathematical structure.

4. **The cubic function was the real bottleneck.** `rentabilidad_por_propiedad_mes` had genuinely poor complexity that grows with data. This is where optimization effort should focus.
