# Algorithm Efficiency Analysis Report

## File: `eval1_subtle_tradeoffs.rs`

Analysis follows the 6-step systematic workflow from the Algorithm Advisor skill.

---

## Function 1: `validar_solapamiento_contratos`

### Current Complexity: O(n²) — pairwise comparison

### Assessment: **NO OPTIMIZATION NEEDED**

**Reasoning:**

The function's own documentation states it operates on 3–8 contracts per propiedad. With n ≤ 8, the O(n²) pairwise comparison performs at most 28 comparisons (8 choose 2). This is trivially fast.

The "optimized" alternative (sort by `fecha_inicio` then scan adjacent pairs) would be O(n log n) — but for n=8, sorting overhead (allocations, comparisons, cache misses from cloning) likely *exceeds* the cost of 28 simple comparisons on a contiguous slice.

Additionally, the sort-then-scan approach only detects overlaps between *adjacent* sorted intervals. If contracts for the same propiedad can have complex overlapping patterns (A overlaps C but not B), the sort-then-scan approach would miss non-adjacent overlaps unless extended to a sweep-line algorithm, adding complexity for zero practical gain.

**Verdict:** The brute-force O(n²) is the correct choice here. Small n, simple code, correct behavior. Premature optimization would add complexity without measurable benefit.

---

## Function 2: `generar_rol_cobros`

### Current Complexity: O(n + m) — HashMap grouping + linear scan per propiedad

### Assessment: **NO OPTIMIZATION NEEDED**

**Reasoning:**

This function already applies the Space-Time Tradeoff paradigm correctly:
1. Builds a `HashMap<Uuid, Vec<&Contrato>>` in O(m) where m = number of contracts
2. Iterates propiedades in O(n), doing O(1) HashMap lookup per propiedad
3. Filters active contracts within each group — total work across all groups is O(m)

Total complexity: O(n + m). This is optimal for this problem. The code is clean, idiomatic, and uses the correct data structure. Called once per month with ~50 propiedades and ~200 contracts — no performance concern whatsoever.

**Verdict:** Already optimal. No changes warranted.

---

## Function 3: `propiedades_completamente_ocupadas`

### Current Complexity: O(n × k) where n = propiedades, k = avg units per propiedad

### Assessment: **NO OPTIMIZATION NEEDED**

**Reasoning:**

With ~50 propiedades and 1–20 units each, worst case is 50 × 20 = 1,000 string comparisons. The code uses `.all()` which short-circuits on the first non-match, so average case is much better.

A "clever" approach (pre-indexing unit states into a HashMap or bitset) would:
- Add memory overhead for the index structure
- Add code complexity
- Provide zero measurable speedup for 1,000 comparisons

The iterator chain is idiomatic Rust, reads clearly, and performs well. The short-circuit behavior of `.all()` is already the optimal strategy for this check.

**Verdict:** Already efficient. Clean idiomatic code with short-circuit evaluation. No changes warranted.

---

## Function 4: `contratos_por_vencer`

### Current Complexity: O(n) — single linear scan with filter

### Assessment: **NO OPTIMIZATION NEEDED**

**Reasoning:**

A linear scan over ~200 contracts with a simple date range filter is O(n) — you cannot do better than this without pre-sorted/indexed data that persists across calls.

The suggested BTreeMap approach would:
- Require maintaining a separate BTreeMap indexed by `fecha_fin` alongside the primary data
- Add O(log n) insertion cost on every contract creation/update
- Provide O(log n) range query — but for n=200, the constant factors of BTree traversal vs. a simple linear scan are comparable
- Add architectural complexity (maintaining a secondary index in sync with the primary data)

For a daily background job on 200 elements, a linear scan completes in microseconds. The BTreeMap would only be justified if n were in the tens of thousands AND the query ran frequently (not once daily).

**Verdict:** Linear scan is the right approach. Simple, correct, fast enough. The BTreeMap suggestion is over-engineering for this dataset size and call frequency.

---

## Function 5: `ingreso_mensual_por_moneda`

### Current Complexity: O(n) — single pass with HashMap accumulation

### Assessment: **NO OPTIMIZATION NEEDED**

**Reasoning:**

This is a textbook single-pass aggregation:
1. Filter active contracts (lazy, via iterator)
2. Accumulate into HashMap by currency key

With ~200 contracts and likely 2 currency keys (DOP, USD), this is O(n) time and O(k) space where k is the number of distinct currencies (effectively constant). This is the optimal algorithm for this problem — you must examine every active contract at least once.

The code is clean, idiomatic, and uses the correct data structure (HashMap for accumulation by key).

**Verdict:** Already optimal. Textbook O(n) aggregation. No changes warranted.

---

## Summary

| Function | Current Big-O | Optimization Needed? | Reason |
|----------|--------------|---------------------|--------|
| `validar_solapamiento_contratos` | O(n²) | **No** | n ≤ 8; brute force is faster than sort overhead |
| `generar_rol_cobros` | O(n + m) | **No** | Already uses Space-Time Tradeoff (HashMap grouping) |
| `propiedades_completamente_ocupadas` | O(n × k) | **No** | Short-circuit `.all()` on small dataset (≤1000 ops) |
| `contratos_por_vencer` | O(n) | **No** | Linear scan is optimal for n=200, daily frequency |
| `ingreso_mensual_por_moneda` | O(n) | **No** | Textbook single-pass aggregation, already optimal |

### Overall Assessment

All five functions in this file are either already algorithmically optimal or operate on datasets small enough that optimization would be premature. The code demonstrates good practices:

- `generar_rol_cobros` correctly applies the Space-Time Tradeoff paradigm with HashMap pre-grouping
- `propiedades_completamente_ocupadas` uses idiomatic iterator chains with short-circuit evaluation
- `contratos_por_vencer` and `ingreso_mensual_por_moneda` are clean O(n) single-pass algorithms
- `validar_solapamiento_contratos` uses brute force appropriately for its documented constraint (n ≤ 8)

**No code changes are warranted.** The output file is identical to the input.
