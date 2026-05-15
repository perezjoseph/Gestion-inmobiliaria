# Algorithm & Data Structure Optimization Report

## Summary

Reviewed `eval3_collection_choice.rs` for suboptimal data structure choices and algorithm inefficiencies. Found 5 issues across 5 functions plus 1 struct, all fixed by matching collection types and algorithms to actual access patterns.

---

## Issues Found & Fixes Applied

### 1. `buscar_inquilino_por_cedula` — Linear scan on Vec

| Aspect | Before | After |
|--------|--------|-------|
| Data structure | `&[Inquilino]` with `.find()` | `&HashMap<String, Inquilino>` with `.get()` |
| Complexity | O(n) per lookup | O(1) amortized per lookup |
| Pattern mismatch | Repeated key-based lookups on unsorted slice | Hash index built once, queried many times |

**Fix:** Changed the function signature to accept a pre-built `HashMap<String, Inquilino>` indexed by cédula. Added a helper `construir_indice_cedula()` to build the index once in O(n), enabling all subsequent lookups in O(1).

---

### 2. `cedulas_duplicadas` — O(n×m) nested iteration

| Aspect | Before | After |
|--------|--------|-------|
| Algorithm | For each new cédula, scan all inquilinos | Build HashSet of existing cédulas, then check membership |
| Complexity | O(n × m) | O(n + m) |
| Pattern mismatch | Membership testing done via linear scan | HashSet provides O(1) membership test |

**Fix:** Pre-collect existing cédulas into a `HashSet<&str>`, then filter new cédulas against the set. Reduces from quadratic to linear.

---

### 3. `contratos_por_inquilino` — BTreeMap without ordering requirement

| Aspect | Before | After |
|--------|--------|-------|
| Data structure | `BTreeMap<Uuid, Vec<&ContratoActivo>>` | `HashMap<Uuid, Vec<&ContratoActivo>>` |
| Insert/lookup | O(log n) | O(1) amortized |
| Pattern mismatch | BTreeMap maintains sorted order; code never uses ordering | HashMap matches unordered access pattern |

**Fix:** Replaced `BTreeMap` with `HashMap` and added `with_capacity` pre-allocation. The result is only accessed via `.get()` and unordered iteration — no sorting needed.

---

### 4. `ingreso_por_propiedad` — O(n²) redundant filtering + BTreeMap

| Aspect | Before | After |
|--------|--------|-------|
| Algorithm | For each contrato, filter ALL contratos by propiedad_id, collect into Vec, then sum | Single pass: accumulate directly into HashMap |
| Complexity | O(n²) | O(n) |
| Data structure | BTreeMap (O(log n) ops, ordering unused) | HashMap (O(1) amortized ops) |
| Intermediate allocation | Allocates a `Vec<f64>` per iteration | Zero intermediate allocations |

**Fix:** Single-pass accumulation with `entry().or_default() += monto`. Eliminated the nested filter, intermediate Vec collection, and replaced BTreeMap with HashMap.

---

### 5. `top_inquilinos` — Full sort for partial selection

| Aspect | Before | After |
|--------|--------|-------|
| Algorithm | Full sort O(n log n) then truncate | Min-heap of size k: O(n log k) |
| Data structure | BTreeMap for accumulation | HashMap for accumulation + BinaryHeap for selection |
| When k << n | Wastes O(n log n) sorting elements we discard | Only maintains k elements in heap |

**Fix:** Replaced BTreeMap with HashMap for the accumulation phase. Used a `BinaryHeap<Reverse<...>>` (min-heap) of capacity `n` for partial selection. For top-k problems where k << n, this is O(n log k) vs O(n log n). Added an `OrdF64Entry` wrapper to handle f64 ordering in the heap.

---

### 6. `ColaSolicitudes` — Vec with insert(0) for FIFO queue

| Aspect | Before | After |
|--------|--------|-------|
| Data structure | `Vec<String>` | `VecDeque<String>` |
| Enqueue | `insert(0, ...)` — O(n) shifts all elements | `push_back()` — O(1) amortized |
| Dequeue | `pop()` from back — O(1) | `pop_front()` — O(1) amortized |
| Pattern mismatch | Vec is optimized for stack (LIFO), not queue (FIFO) | VecDeque is a ring buffer designed for FIFO |

**Fix:** Replaced `Vec` with `VecDeque`. Enqueue uses `push_back()`, dequeue uses `pop_front()`. Both operations are O(1) amortized.

---

## Complexity Summary

| Function | Before | After | Speedup Factor (for n items) |
|----------|--------|-------|------------------------------|
| `buscar_inquilino_por_cedula` | O(n) | O(1) | n× |
| `cedulas_duplicadas` | O(n×m) | O(n+m) | min(n,m)× |
| `contratos_por_inquilino` | O(n log n) | O(n) | log(n)× |
| `ingreso_por_propiedad` | O(n²) | O(n) | n× |
| `top_inquilinos` | O(n log n) | O(n log k) | log(n)/log(k)× |
| `ColaSolicitudes::encolar` | O(n) | O(1) | n× |

## Design Decisions

- **HashMap over BTreeMap**: Used wherever ordering is not required. HashMap provides O(1) amortized operations vs O(log n) for BTreeMap.
- **HashSet for membership testing**: Natural choice when the only operation is "does this element exist?"
- **VecDeque for FIFO**: Ring buffer with O(1) operations at both ends, purpose-built for queue semantics.
- **BinaryHeap for top-k**: Partial selection avoids sorting elements that won't appear in the result.
- **`with_capacity` pre-allocation**: Used on HashMaps and BinaryHeap to reduce reallocations when the size is known or bounded.
- **Index builder pattern**: Separated index construction (`construir_indice_cedula`) from lookup to make the O(n) build cost explicit and amortizable across multiple queries.
