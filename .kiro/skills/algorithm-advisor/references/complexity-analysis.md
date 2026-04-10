# Complexity Analysis Reference Guide

## Big-O Notation Quick Reference

| Notation | Name | Example Operation |
|----------|------|-------------------|
| O(1) | Constant | `HashMap::get`, `Vec::push` (amortized), `Vec::pop` |
| O(log n) | Logarithmic | `BTreeMap::get`, binary search on sorted `Vec` |
| O(n) | Linear | `Vec::contains`, `Iterator::find`, single-pass filter/map |
| O(n log n) | Linearithmic | `Vec::sort`, `Vec::sort_by_key` |
| O(n²) | Quadratic | Nested loop over two collections, pairwise comparisons |
| O(n·m) | Bilinear | Nested loop over two different-sized collections |

### Space Complexity Basics

| Pattern | Space | Notes |
|---------|-------|-------|
| In-place sort | O(1) extra | `slice::sort` uses O(log n) stack for recursion |
| `Iterator::collect::<Vec<_>>()` | O(n) | Allocates a new Vec with all results |
| `HashMap` from n items | O(n) | Hash table overhead ~1.5× element count |
| Chained iterators (no collect) | O(1) | Lazy evaluation, no intermediate allocation |


## Amortized Analysis for Vec::push

`Vec::push` is O(1) amortized despite occasional O(n) reallocations. When the internal buffer is full, `Vec` doubles its capacity and copies all elements. Over a sequence of n pushes, the total cost of all copies is bounded by 2n, giving O(1) per push on average.

### Why This Matters

Avoid pre-optimizing `Vec::push` in loops unless profiling shows reallocation as a bottleneck. The amortized cost is already efficient for most use cases.

### When to Use `with_capacity`

Pre-allocate when you know the final size to eliminate all reallocations:

```rust
// Anti-pattern: unknown capacity, multiple reallocations for large result sets
let mut pagos_pendientes: Vec<&pago::Model> = Vec::new();
for pago in &pagos {
    if pago.estado == "pendiente" {
        pagos_pendientes.push(pago);
    }
}

// Better: pre-allocate upper bound when size is predictable
let mut pagos_pendientes: Vec<&pago::Model> = Vec::with_capacity(pagos.len());
for pago in &pagos {
    if pago.estado == "pendiente" {
        pagos_pendientes.push(pago);
    }
}

// Best: use iterator chain — collects directly with size hint
let pagos_pendientes: Vec<&pago::Model> = pagos
    .iter()
    .filter(|p| p.estado == "pendiente")
    .collect();
```

The iterator chain is preferred because `.collect()` uses the iterator's `size_hint()` to pre-allocate, and the code is more idiomatic.

### Reallocation Cost Table

| Pushes | Reallocations | Total Copies |
|--------|---------------|--------------|
| 8 | 3 | 7 |
| 64 | 6 | 63 |
| 1,024 | 10 | 1,023 |
| 1,000,000 | 20 | ~1,000,000 |

For collections under ~1,000 elements (typical in property management queries), reallocation overhead is negligible. Use `with_capacity` for batch operations processing thousands of records.



## Common Anti-Patterns

### Nested `.contains()` Loops — O(n·m)

The most frequent quadratic pattern in domain code: checking membership inside a loop using `Vec::contains` or `Iterator::any`.

**Anti-pattern — checking active tenants against contracts:**

```rust
// O(n·m): for each contrato, scan the entire inquilinos Vec
let cedulas_activas: Vec<String> = inquilinos
    .iter()
    .map(|i| i.cedula.clone())
    .collect();

for contrato in &contratos {
    // O(n) linear scan per iteration
    if cedulas_activas.contains(&contrato_cedula) {
        // process active tenant contract
    }
}
```

**Fix — HashSet for O(1) membership testing:**

```rust
use std::collections::HashSet;

// O(n) to build the set
let cedulas_activas: HashSet<&str> = inquilinos
    .iter()
    .map(|i| i.cedula.as_str())
    .collect();

// O(m) total: O(1) per lookup
for contrato in &contratos {
    if cedulas_activas.contains(contrato_cedula.as_str()) {
        // process active tenant contract
    }
}
// Total: O(n + m) instead of O(n·m)
```

**Detection heuristic:** Any `.contains()`, `.iter().any()`, or `.iter().find()` call on a `Vec` or slice inside a `for` loop or `.iter().filter()` is a candidate for replacement with a `HashSet` or `HashMap` pre-built outside the loop.

### Repeated `.find()` on Unsorted Data — O(n·k)

Calling `.find()` or `.iter().position()` multiple times on the same unsorted collection multiplies the linear scan cost.

**Anti-pattern — looking up property details for each contract:**

```rust
// O(n·k): for each of k contratos, scan all n propiedades
for contrato in &contratos {
    let propiedad = propiedades
        .iter()
        .find(|p| p.id == contrato.propiedad_id); // O(n) each time

    if let Some(prop) = propiedad {
        // use prop.direccion, prop.ciudad, etc.
    }
}
```

**Fix — build a HashMap lookup table once:**

```rust
use std::collections::HashMap;

// O(n) to build the lookup
let propiedades_por_id: HashMap<Uuid, &propiedad::Model> = propiedades
    .iter()
    .map(|p| (p.id, p))
    .collect();

// O(k) total: O(1) per lookup
for contrato in &contratos {
    if let Some(prop) = propiedades_por_id.get(&contrato.propiedad_id) {
        // use prop.direccion, prop.ciudad, etc.
    }
}
// Total: O(n + k) instead of O(n·k)
```

**Detection heuristic:** Multiple `.find()` or `.iter().find()` calls on the same collection with the same key field, especially inside a loop. Build a `HashMap` keyed on the lookup field before the loop.

### Unnecessary Sort for Min/Max — O(n log n) vs O(n)

Sorting an entire collection just to get the first or last element wastes work.

**Anti-pattern — finding the latest payment:**

```rust
// O(n log n) sort to find a single element
let mut pagos_sorted = pagos.clone();
pagos_sorted.sort_by_key(|p| p.fecha_vencimiento);
let ultimo_pago = pagos_sorted.last();
```

**Fix — use iterator min/max:**

```rust
// O(n) single pass, no allocation, no clone
let ultimo_pago = pagos.iter().max_by_key(|p| p.fecha_vencimiento);
```

**When sorting IS appropriate:**
- You need the top-k elements (sort + truncate, or use a `BinaryHeap`)
- You need the full sorted order for display or sequential processing
- You need to detect adjacent duplicates or overlaps (see contract overlap below)

### Collect-Then-Iterate — Unnecessary Intermediate Allocation

Building an intermediate `Vec` only to iterate over it immediately wastes memory.

**Anti-pattern:**

```rust
// Allocates an intermediate Vec just to sum
let montos: Vec<Decimal> = pagos
    .iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .collect(); // unnecessary allocation

let total: Decimal = montos.iter().sum();
```

**Fix — chain directly:**

```rust
// No intermediate allocation, single pass
let total: Decimal = pagos
    .iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .sum();
```

**Detection heuristic:** `.collect::<Vec<_>>()` immediately followed by `.iter()` on the result. The intermediate `Vec` can almost always be eliminated by continuing the iterator chain.



## Contract Overlap Detection: Algorithmic Alternatives

Detecting overlapping date ranges is a core business rule: a property cannot have overlapping active contracts. Two date ranges `[a_start, a_end]` and `[b_start, b_end]` overlap when `a_start <= b_end AND b_start <= a_end`.

### Approach 1: Brute Force Pairwise — O(n²)

Compare every pair of contracts for the same property.

```rust
// O(n²) — checks all pairs
fn find_overlaps_brute(contratos: &[contrato::Model]) -> Vec<(Uuid, Uuid)> {
    let mut overlaps = Vec::new();
    for i in 0..contratos.len() {
        for j in (i + 1)..contratos.len() {
            if contratos[i].propiedad_id == contratos[j].propiedad_id
                && contratos[i].fecha_inicio <= contratos[j].fecha_fin
                && contratos[j].fecha_inicio <= contratos[i].fecha_fin
            {
                overlaps.push((contratos[i].id, contratos[j].id));
            }
        }
    }
    overlaps
}
```

Only acceptable for very small datasets (< 20 contracts per property). Becomes a bottleneck when validating across all properties at once.

### Approach 2: Sort-Then-Scan — O(n log n)

Sort contracts by `fecha_inicio`, then check only adjacent pairs. After sorting, if contract `i` overlaps with contract `i+1`, their date ranges must be adjacent in the sorted order.

```rust
use std::collections::HashMap;

/// Returns pairs of overlapping contract IDs, grouped by property.
/// Complexity: O(n log n) dominated by the sort.
fn find_overlaps_sorted(contratos: &[contrato::Model]) -> Vec<(Uuid, Uuid)> {
    // Group by propiedad_id — O(n)
    let mut por_propiedad: HashMap<Uuid, Vec<&contrato::Model>> = HashMap::new();
    for c in contratos {
        por_propiedad.entry(c.propiedad_id).or_default().push(c);
    }

    let mut overlaps = Vec::new();

    for (_propiedad_id, mut grupo) in por_propiedad {
        // Sort by fecha_inicio — O(k log k) where k = contracts per property
        grupo.sort_by_key(|c| c.fecha_inicio);

        // Scan adjacent pairs — O(k)
        for window in grupo.windows(2) {
            if window[0].fecha_fin >= window[1].fecha_inicio {
                overlaps.push((window[0].id, window[1].id));
            }
        }
    }

    overlaps
}
```

**Why this works:** After sorting by `fecha_inicio`, any overlap must occur between consecutive contracts. If contract A ends before contract B starts, no later contract C (with `C.fecha_inicio >= B.fecha_inicio`) can overlap with A either.

**Limitation:** Only detects overlaps between adjacent pairs. If contract A spans a very long range overlapping both B and C, the scan catches A↔B and B↔C but may miss A↔C. For the property management domain this is sufficient — if A overlaps B and B overlaps C, all three are flagged as problematic.

**Best for:** Single-property validation (e.g., when creating or updating a contract). This is the recommended approach for this project.

### Approach 3: Interval Tree — O(n log n) Build, O(log n + k) Query

An interval tree stores date ranges in a balanced BST, enabling efficient overlap queries. Each query returns all overlapping intervals in O(log n + k) where k is the number of results.

```rust
// Conceptual structure — requires a crate like `rudac` or custom implementation
// Each node stores an interval [fecha_inicio, fecha_fin] and a max endpoint
// in its subtree for efficient pruning.

struct IntervalNode {
    contrato_id: Uuid,
    fecha_inicio: Date,
    fecha_fin: Date,
    max_fin: Date, // max fecha_fin in this subtree
}

// Query: find all contracts overlapping [query_start, query_end]
// 1. If node.fecha_inicio > query_end → skip right subtree
// 2. If node.max_fin < query_start → skip this subtree entirely
// 3. Otherwise check overlap and recurse both children
```

**When to use an interval tree:**
- Frequent overlap queries against a large, mostly-static set of contracts
- Need to answer "which contracts overlap this date range?" repeatedly
- Dataset exceeds ~10,000 contracts per property (unlikely in property management)

**When NOT to use:**
- One-off validation when creating a single contract (sort-then-scan is simpler)
- Small datasets where the constant factor of tree operations exceeds brute force
- The collection changes frequently (tree rebalancing adds overhead)

### Comparison Table

| Approach | Build | Query (single) | Query (all pairs) | Space | Best For |
|----------|-------|-----------------|--------------------|----|----------|
| Brute force | — | O(n²) | O(n²) | O(1) | n < 20 |
| Sort-then-scan | O(n log n) | O(n) | O(n log n) | O(n) | Single-property validation |
| Interval tree | O(n log n) | O(log n + k) | O(n log n + k) | O(n) | Repeated queries on static data |

**Recommendation for this project:** Use sort-then-scan. Property management typically has a small number of contracts per property (< 50), and overlap checks happen during contract creation/update — a one-off validation. The sort-then-scan approach is simple, efficient, and requires no external crates.

## Complexity Cheat Sheet for Domain Operations

| Operation | Naive Approach | Optimized Approach | Improvement |
|-----------|---------------|-------------------|-------------|
| Find tenant by cedula in list | `Vec::iter().find()` — O(n) | `HashMap<&str, &Model>` — O(1) | O(n) → O(1) |
| Check if cedula exists | `Vec::contains()` — O(n) | `HashSet<&str>` — O(1) | O(n) → O(1) |
| Payments for a contract | `Vec::iter().filter()` — O(n) per query | `HashMap<Uuid, Vec<&Model>>` — O(1) lookup | O(n·k) → O(n+k) |
| Contract overlap detection | Pairwise comparison — O(n²) | Sort by fecha_inicio + scan — O(n log n) | O(n²) → O(n log n) |
| Latest payment for contract | Sort + last — O(n log n) | `Iterator::max_by_key` — O(n) | O(n log n) → O(n) |
| Total paid amount | Collect + sum — O(n) + alloc | `Iterator::filter().map().sum()` — O(n) | Same complexity, zero allocation |
| Properties by ciudad | Nested loop search — O(n·m) | `HashMap<&str, Vec<&Model>>` — O(1) lookup | O(n·m) → O(n+m) |
