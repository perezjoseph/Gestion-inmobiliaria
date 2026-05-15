# Algorithm Optimization Report: eval3_paradigm_selection.rs

## Summary

Each function in this file requires a **different** optimization paradigm. The key insight is that not every function needs optimization — recognizing when code is already optimal is itself a paradigm decision.

| Function | Original Complexity | Optimized Complexity | Paradigm | Changed? |
|----------|-------------------|---------------------|----------|----------|
| `aplicar_pago_parcial` | O(n log n) | O(n log n) | Greedy (keep) | No |
| `rentabilidad_por_propiedad_mes` | O(P × M × (pagos + gastos)) | O(pagos + gastos + P × M) | Transform and Conquer | Yes |
| `mejor_mes_mantenimiento` | O(12 × n) = O(n) | O(n) | Brute Force (keep) | No |
| `cascada_morosidad` | O(months × pagos) | O(pagos + months) | Decrease and Conquer (prefix sum) | Yes |

---

## Function 1: `aplicar_pago_parcial`

### Paradigm: Greedy — Keep As-Is

**Why this paradigm fits:**
The greedy approach (sort by date, allocate oldest-first) is the textbook solution for this type of allocation problem. The business rule explicitly requires oldest-first ordering, which means we MUST process items in sorted order.

**Why a min-heap is NOT better:**
- A min-heap gives O(n) construction, but extracting all elements in order is O(n log n) — same as sort.
- With n = 1–12 items, the constant factors of heap operations (pointer chasing, cache misses) actually make it SLOWER than a contiguous array sort.
- After allocation, the caller receives a sorted slice — useful for display. A heap would destroy this property.
- The sort is in-place on a mutable slice, which is the most cache-friendly option.

**Decision: No change.** The code is already optimal for its dataset size and access pattern.

---

## Function 2: `rentabilidad_por_propiedad_mes`

### Paradigm: Transform and Conquer

**The problem with the original:**
The original has three nested dimensions: for each propiedad (50), for each month (24), it scans all pagos (2000) and all gastos (500), AND for each pago it scans all contratos to find the propiedad. This is effectively O(50 × 24 × 2000 × contratos) — quartic in the worst case.

**Why Transform and Conquer fits:**
The core insight is that we can TRANSFORM the input data into indexed structures that make the final computation trivial:

1. **Transform phase** (one-time cost):
   - Build `contrato_id → propiedad_id` map: O(contratos)
   - Index pagos by composite key `(propiedad_id, month)`: O(pagos)
   - Index gastos by composite key `(propiedad_id, month)`: O(gastos)

2. **Conquer phase** (assembly):
   - For each (propiedad, month) pair, do O(1) HashMap lookups instead of O(n) scans.

**Why a simple HashMap won't work alone:**
A single-dimension HashMap (e.g., by propiedad_id only) still requires scanning within each bucket to filter by month. The key insight is using a COMPOSITE key `(Uuid, String)` that indexes both dimensions simultaneously.

**Why alternatives don't fit:**
- **Divide and Conquer**: No natural way to split this problem into independent subproblems.
- **Dynamic Programming**: No overlapping subproblems or optimal substructure.
- **Greedy**: Not an optimization problem — we need exact totals, not approximations.

**Complexity improvement:** From O(P × M × N) to O(N + P × M) where N = pagos + gastos.

---

## Function 3: `mejor_mes_mantenimiento`

### Paradigm: Brute Force — Keep As-Is

**Why this paradigm fits:**
The outer loop is FIXED at 12 iterations (next 12 months). This makes the algorithm O(12n) = O(n) where n = 200 contracts. Total work: ~2400 comparisons — trivial.

**Why an interval tree is NOT warranted:**
- Interval tree construction is O(n log n) with significant constant overhead (tree allocation, balancing).
- A sweep line algorithm requires sorting events: O(n log n) setup for O(n) sweep.
- Both are designed for problems where the number of QUERIES is large relative to n. Here we have exactly 12 queries.
- The break-even point where an interval tree wins is roughly when queries > log(n). With 12 queries and n=200, log₂(200) ≈ 8, so we're barely past break-even — and the constant factors of tree operations make it slower in practice.

**Decision: No change.** The code is already effectively O(n) with excellent cache locality (linear scan of a contiguous slice). Any "optimization" would make it slower and harder to read.

---

## Function 4: `cascada_morosidad`

### Paradigm: Decrease and Conquer (Prefix Sum)

**The problem with the original:**
For each month M, the original scans ALL payments to find those with `fecha_vencimiento < M` that are delinquent. This is O(months × pagos) = O(24 × 2000) = 48,000 operations, with redundant string formatting on every comparison.

**Why Decrease and Conquer / Prefix Sum fits:**
The cumulative delinquency for month M has a recursive structure:

```
cascade(M) = cascade(M-1) + delinquent_count(M-1)
```

Each month's answer BUILDS ON the previous month's answer. This is the hallmark of decrease-and-conquer: reduce the problem by one unit and use the smaller solution.

**Implementation:**
1. Single pass over payments: bucket delinquent counts by month → O(pagos)
2. Prefix sum over months: each month's cascade = running total of prior months → O(months)

**Why alternatives don't fit:**
- **HashMap grouping alone**: Groups payments by month but doesn't capture the "from PREVIOUS months" cumulative semantics. You'd still need a nested loop to sum prior months.
- **Transform and Conquer**: Overkill — we don't need multi-dimensional indexing, just a 1D accumulation.
- **Divide and Conquer**: The cumulative nature means subproblems aren't independent.

**Complexity improvement:** From O(months × pagos) to O(pagos + months) — linear.

---

## Design Decisions

1. **Used `BTreeSet`/`BTreeMap` for months** — gives us sorted order for free without a separate sort step, which is needed for correct prefix-sum ordering and chronological output.

2. **Composite HashMap keys `(Uuid, String)`** in `rentabilidad_por_propiedad_mes` — avoids nested HashMaps which would require two lookups and more allocations.

3. **`Vec::with_capacity`** where the size is known — avoids reallocations per the project's code style guidelines.

4. **Preserved function signatures** — all public APIs remain unchanged, ensuring drop-in compatibility.
