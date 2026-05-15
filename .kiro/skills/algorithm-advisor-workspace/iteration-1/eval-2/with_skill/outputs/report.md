# Algorithm Optimization Report: Dashboard Matching

## Summary

Optimized three functions in `eval2_dashboard_matching.rs` that exhibited O(n²) and O(n³) patterns from nested loops matching propiedades, contratos, and pagos. All optimizations apply the **Space-Time Tradeoff** paradigm — precomputing HashMap/HashSet lookup tables to eliminate nested linear scans.

## Changes Applied

### 1. `generar_resumen` — Triple Nested Loop Elimination

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(p × c × g) | O(p + c + g) |
| Space Complexity | O(1) extra | O(c + g) for lookup maps |
| Paradigm | Brute Force | Space-Time Tradeoff |

**What changed:**
- Built `pagos_by_contrato: HashMap<Uuid, Vec<&Pago>>` in O(g) — groups pagos by their contrato_id
- Built `contratos_by_propiedad: HashMap<Uuid, Vec<&Contrato>>` in O(c) — groups contratos by their propiedad_id
- Main loop iterates propiedades once, using O(1) HashMap lookups instead of scanning all contratos and all pagos per propiedad
- Added `Vec::with_capacity(propiedades.len())` to avoid reallocations

**Rationale:** The original triple-nested loop scanned all contratos for every propiedad, and all pagos for every matching contrato. With n propiedades, m contratos, and k pagos, this is O(n × m × k). The HashMap approach processes each entity exactly once during map construction, then uses O(1) lookups during aggregation.

---

### 2. `propiedades_con_atrasos` — Set-Based Filtering

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(p × c × g) | O(p + c + g) |
| Space Complexity | O(1) extra | O(c + p) for HashSets |
| Paradigm | Brute Force | Space-Time Tradeoff |

**What changed:**
- Built `contratos_con_atraso: HashSet<Uuid>` by filtering pagos with estado "atrasado" — O(g)
- Built `propiedades_con_atraso: HashSet<Uuid>` by filtering active contratos that appear in the atraso set — O(c)
- Final pass filters propiedades by membership in the set — O(p)
- Preserves input order of propiedades in the result

**Rationale:** The original code used three nested loops with early-break logic. While the breaks help in the best case, worst case remains O(p × c × g). The set-based approach guarantees linear time regardless of data distribution.

---

### 3. `ocupacion_por_ciudad` — Single-Pass Aggregation

| Metric | Before | After |
|--------|--------|-------|
| Time Complexity | O(cities × n) | O(n + cities × log(cities)) |
| Space Complexity | O(cities) for deduped vec | O(cities) for HashMap |
| Paradigm | Brute Force (re-scan per city) | Space-Time Tradeoff |

**What changed:**
- Replaced the pattern of collecting unique cities then re-scanning propiedades per city
- Single pass over propiedades accumulates `(total, ocupadas)` counts per city in a HashMap
- Final sort on city names preserves deterministic output order (equivalent to original's sort+dedup behavior)

**Rationale:** The original scanned all propiedades once to collect cities, then scanned all propiedades again for *each* city. If there are k cities and n propiedades, this is O(n + k×n) = O(k×n). The HashMap approach processes each propiedad exactly once in O(n), then sorts only the k city keys in O(k log k). Since k ≤ n, total is O(n + k log k) ≈ O(n).

---

## Tradeoff Assessment

| Factor | Assessment |
|--------|------------|
| Space overhead | Proportional to input size (HashMap entries = number of contratos/pagos). Acceptable for dashboard queries. |
| Correctness | Semantically equivalent — same results, same edge case handling. Output order preserved where relevant. |
| Maintainability | Improved — each function has clear phases (build index → aggregate) instead of deeply nested conditionals. |
| When n is small | The optimization still helps readability. HashMap overhead is negligible for small n. |

## Paradigm Applied

All three optimizations use **Space-Time Tradeoff** (Paradigm #7):
- Trade O(n) memory for lookup tables
- Reduce O(n²) or O(n³) nested scans to O(n) with O(1) amortized HashMap/HashSet lookups
- This is the most commonly applicable paradigm for the "nested loop with inner search" anti-pattern
