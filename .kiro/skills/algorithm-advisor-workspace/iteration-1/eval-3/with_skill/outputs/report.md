# Algorithm Advisor Report: eval3_collection_choice.rs

## Summary

Identified and fixed **6 algorithm/data structure inefficiencies** across 5 functions and 1 struct. All fixes follow the systematic optimization workflow from the Algorithm Advisor skill.

## Changes Applied

### 1. `buscar_inquilino_por_cedula` — Space-Time Tradeoff

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n) per lookup | O(1) amortized per lookup |
| Paradigm | — | Space-Time Tradeoff (precomputed index) |

**Problem:** Linear scan through `&[Inquilino]` on every call. If called k times, total cost is O(n·k).

**Fix:** Introduced `construir_indice_cedulas()` that builds a `HashMap<&str, &Inquilino>` once in O(n). Subsequent lookups are O(1) amortized. The API now takes the prebuilt index, encouraging callers to build it once and reuse.

**Rationale:** This is the textbook space-time tradeoff — trade O(n) memory for O(1) lookup. Justified whenever the lookup is called more than once.

---

### 2. `cedulas_duplicadas` — Space-Time Tradeoff (HashSet)

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n·m) | O(n + m) |
| Paradigm | — | Space-Time Tradeoff (HashSet membership) |

**Problem:** For each of m new cédulas, scans all n inquilinos — classic nested loop pattern.

**Fix:** Build a `HashSet<&str>` from existing cédulas in O(n), then check each new cédula in O(1). Total: O(n + m).

**Rationale:** HashSet provides O(1) membership testing. No ordering needed for a "does it exist?" check.

---

### 3. `contratos_por_inquilino` — Transform and Conquer (Representation Change)

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n log n) total insertions | O(n) amortized total insertions |
| Paradigm | — | Transform & Conquer (representation change) |

**Problem:** Used `BTreeMap` which maintains sorted order at O(log n) per operation. The result is only accessed via `.get()` and `.iter()` without any ordering requirement.

**Fix:** Replaced with `HashMap` for O(1) amortized insert and lookup. Added `with_capacity` to reduce reallocations.

**Rationale:** BTreeMap's O(log n) overhead is wasted when ordering is never consumed. HashMap is the correct choice for unordered key-value access.

---

### 4. `ingreso_por_propiedad` — Space-Time Tradeoff + Eliminate Unnecessary Work

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n²) | O(n) |
| Paradigm | — | Space-Time Tradeoff (single-pass accumulation) |

**Problem:** Two issues compounded:
1. For each contrato, re-scans ALL contratos filtering by `propiedad_id` — O(n²).
2. Collects filtered results into an intermediate `Vec<f64>` before summing — unnecessary allocation.
3. Used `BTreeMap` when ordering is not needed.

**Fix:** Single-pass accumulation into a `HashMap<Uuid, f64>` with `with_capacity`. Each contrato's `monto_mensual` is added directly to its property's running total. No intermediate allocation, no redundant filtering.

**Rationale:** The original code recomputed the same sum for the same `propiedad_id` multiple times. A single pass with accumulation is the canonical O(n) solution.

---

### 5. `top_inquilinos` — Decrease and Conquer (Partial Sort via Min-Heap)

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n log n) full sort | O(n + k log k) partial sort |
| Paradigm | — | Decrease & Conquer (selection via min-heap) |

**Problem:** Sorted the entire collection just to take the first N elements. Also used `BTreeMap` unnecessarily.

**Fix:**
1. Replaced `BTreeMap` with `HashMap` for accumulation (no ordering needed during aggregation).
2. Used a `BinaryHeap` (min-heap via `Reverse`) of size k to maintain only the top-N elements. This avoids sorting the entire dataset.

**Rationale:** When k << n, a min-heap of size k gives O(n log k) which is significantly better than O(n log n) full sort. The final k-element sort is O(k log k) which is negligible.

**Note:** Introduced an `OrderedFloat` wrapper to handle f64's lack of `Ord` in a safe manner (NaN treated as equal).

---

### 6. `ColaSolicitudes` — Transform and Conquer (Representation Change)

| Metric | Before | After |
|--------|--------|-------|
| Complexity | O(n) per enqueue | O(1) amortized per enqueue |
| Paradigm | — | Transform & Conquer (representation change) |

**Problem:** Used `Vec` with `insert(0, ...)` for FIFO semantics. Every insertion shifts all existing elements — O(n) per operation.

**Fix:** Replaced with `VecDeque` which provides O(1) amortized `push_back` and `pop_front`. Changed the enqueue to `push_back` and dequeue to `pop_front` to maintain correct FIFO ordering.

**Rationale:** `VecDeque` is Rust's standard double-ended queue, purpose-built for this access pattern. It uses a ring buffer internally, avoiding element shifts entirely.

---

## Paradigm Usage Summary

| Paradigm | Times Applied | Functions |
|----------|--------------|-----------|
| Space-Time Tradeoff | 3 | `buscar_inquilino_por_cedula`, `cedulas_duplicadas`, `ingreso_por_propiedad` |
| Transform & Conquer (Representation Change) | 2 | `contratos_por_inquilino`, `ColaSolicitudes` |
| Decrease & Conquer (Partial Sort) | 1 | `top_inquilinos` |

## Overall Complexity Improvement

| Function | Before | After |
|----------|--------|-------|
| `buscar_inquilino_por_cedula` | O(n) per call | O(1) per call (after O(n) index build) |
| `cedulas_duplicadas` | O(n·m) | O(n + m) |
| `contratos_por_inquilino` | O(n log n) | O(n) |
| `ingreso_por_propiedad` | O(n²) | O(n) |
| `top_inquilinos` | O(n log n) | O(n + k log k) |
| `ColaSolicitudes::encolar` | O(n) | O(1) amortized |

## Tradeoff Assessment

All optimizations trade a small amount of additional memory (HashMap/HashSet overhead, VecDeque ring buffer) for significant time complexity reductions. Given that these operate on property management data (contracts, tenants, properties) which can scale to thousands of records, the tradeoffs are justified. The code remains idiomatic Rust with clear ownership semantics.

## API Change Note

`buscar_inquilino_por_cedula` now takes a pre-built `HashMap` index instead of a raw slice. This is intentional — it forces callers to build the index once and reuse it, preventing the anti-pattern of rebuilding on every call. The `construir_indice_cedulas` helper makes this ergonomic.
