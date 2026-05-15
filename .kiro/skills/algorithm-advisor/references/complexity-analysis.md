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
| O(2ⁿ) | Exponential | Recursive without memoization, brute-force subsets |
| O(n!) | Factorial | Brute-force permutations |

### Space Complexity Basics

| Pattern | Space | Notes |
|---------|-------|-------|
| In-place sort | O(1) extra | `slice::sort` uses O(log n) stack for recursion |
| `Iterator::collect::<Vec<_>>()` | O(n) | Allocates a new Vec with all results |
| `HashMap` from n items | O(n) | Hash table overhead ~1.5× element count |
| Chained iterators (no collect) | O(1) | Lazy evaluation, no intermediate allocation |
| Memoization cache | O(n) | Trades space for time — stores computed results |
| Prefix sum array | O(n) | Precomputed for O(1) range queries |

## Algorithm Design Paradigms — Detailed Reference

### 1. Brute Force

**What:** Try all possibilities systematically.
**Complexity:** Usually O(n²), O(2ⁿ), or O(n!).
**When acceptable:** Small datasets (< 20 elements), one-off validation, prototyping.
**When to replace:** Any collection > 100 elements with nested search or pairwise comparison.

**Role in the workflow:** Brute force is the baseline. Every optimization is measured against it. Start here to understand the problem, then apply a paradigm to reduce complexity.

### 2. Decrease and Conquer

**What:** Reduce the problem by a constant amount or constant factor each step.
**Three variants:**
- **Decrease by one:** Solve for n-1, extend to n. Insertion sort O(n²), DFS/BFS O(V+E).
- **Decrease by half:** Binary search O(log n), exponentiation by squaring O(log n).
- **Variable decrease:** Euclid's GCD, interpolation search.

**Apply when:** The solution to a smaller instance directly extends to the full problem.

**Domain examples:**

```rust
// Decrease by half: binary search for a payment due date in sorted list
// O(log n) instead of O(n) linear scan
let idx = pagos_sorted
    .binary_search_by_key(&target_date, |p| p.fecha_vencimiento);

// Decrease by one: find max payment amount in single pass
// Each step reduces the unseen portion by 1
let max_pago = pagos.iter().max_by_key(|p| p.monto);
```

### 3. Divide and Conquer

**What:** Split into *independent* subproblems, solve recursively, combine results.
**Complexity:** Determined by the Master Theorem. For recurrence T(n) = aT(n/b) + O(nᵈ):
- If d > log_b(a): O(nᵈ)
- If d = log_b(a): O(nᵈ log n)
- If d < log_b(a): O(n^(log_b(a)))

**Apply when:** The problem naturally splits into independent halves.
**Classic examples:** Merge sort O(n log n), quicksort O(n log n) average, Strassen's matrix multiplication.

**Domain example:**

```rust
// Divide and conquer: process properties in parallel groups by ciudad
// Each group is independent — no cross-group dependencies
let mut por_ciudad: HashMap<&str, Vec<&propiedad::Model>> = HashMap::new();
for p in &propiedades {
    por_ciudad.entry(p.ciudad.as_str()).or_default().push(p);
}
// Solve each subproblem independently
let resultados: Vec<_> = por_ciudad
    .iter()
    .map(|(ciudad, grupo)| (ciudad, compute_stats(grupo)))
    .collect();
```

### 4. Transform and Conquer

**What:** Restructure the problem or data *before* solving.
**Three forms:**

**a) Instance simplification** — Preprocess the input to make the problem easier:

```rust
// Sort contracts by start date, then detect overlaps with adjacent scan
// O(n log n) sort + O(n) scan instead of O(n²) pairwise
let mut sorted = contratos.clone();
sorted.sort_by_key(|c| c.fecha_inicio);
for window in sorted.windows(2) {
    if window[0].fecha_fin >= window[1].fecha_inicio {
        // overlap detected
    }
}
```

**b) Representation change** — Switch to a more efficient data structure:

```rust
// Change Vec to HashMap for O(1) lookups
let propiedades_por_id: HashMap<Uuid, &propiedad::Model> = propiedades
    .iter()
    .map(|p| (p.id, p))
    .collect();
```

**c) Problem reduction** — Reduce to a known solved problem:

```rust
// "Find all overlapping contracts" reduces to "interval intersection"
// "Optimal tenant-unit matching" reduces to "bipartite matching"
// "Payment allocation across debts" reduces to "knapsack variant"
```

### 5. Dynamic Programming

**What:** For problems with *overlapping subproblems* and *optimal substructure*. Store intermediate results to avoid recomputation.
**Complexity:** Reduces exponential to polynomial.
**Two approaches:**
- **Memoization (top-down):** Recursive with cache. Lazy — only computes needed subproblems.
- **Tabulation (bottom-up):** Iterative, fills table from base cases. Eager — computes all subproblems.

**When to apply:**
1. The problem can be broken into subproblems
2. Subproblems overlap (same subproblem solved multiple times)
3. Optimal substructure holds (optimal solution contains optimal sub-solutions)

**Domain example — memoized monthly aggregation:**

```rust
use std::collections::HashMap;

// Without DP: O(n × m) — recomputes monthly totals for each trend point
// With DP: O(n + m) — each month computed once, cached for reuse
let mut cache: HashMap<(i32, u32), Decimal> = HashMap::new();

for &(anio, mes) in &meses_requeridos {
    let total = *cache.entry((anio, mes)).or_insert_with(|| {
        pagos.iter()
            .filter(|p| p.fecha_vencimiento.year() == anio
                && p.fecha_vencimiento.month() == mes
                && p.estado == "pagado")
            .map(|p| p.monto)
            .sum()
    });
    tendencia.push(total);
}
```

**Domain example — tabulation for cumulative income:**

```rust
// Tabulation: build prefix sums for O(1) range queries
// Precompute cumulative income by month
let mut acumulado = Vec::with_capacity(meses.len());
let mut running_total = Decimal::ZERO;
for &(anio, mes) in &meses {
    let mes_total: Decimal = pagos.iter()
        .filter(|p| p.fecha_vencimiento.year() == anio
            && p.fecha_vencimiento.month() == mes
            && p.estado == "pagado")
        .map(|p| p.monto)
        .sum();
    running_total += mes_total;
    acumulado.push(running_total);
}
// Range query: income from month i to month j = acumulado[j] - acumulado[i-1]
```

### 6. Greedy

**What:** Make the locally optimal choice at each step.
**Complexity:** Usually O(n log n) for initial sort + O(n) for greedy pass.
**Correctness requirement:** Must prove the greedy choice property holds — that local optimum leads to global optimum. If you can't prove it, use DP instead.

**Domain example — payment allocation:**

```rust
// Greedy: allocate payment to oldest pending debts first
// Greedy choice: always pay the oldest debt — minimizes late penalties
let mut deudas_sorted = deudas_pendientes.clone();
deudas_sorted.sort_by_key(|d| d.fecha_vencimiento); // O(n log n)

let mut monto_restante = pago.monto;
for deuda in &mut deudas_sorted { // O(n) greedy pass
    if monto_restante <= Decimal::ZERO { break; }
    let aplicar = monto_restante.min(deuda.monto_pendiente);
    deuda.monto_pendiente -= aplicar;
    monto_restante -= aplicar;
}
```

**Domain example — maintenance scheduling by priority:**

```rust
// Greedy: schedule maintenance requests by priority (urgente first)
// Greedy choice: always handle highest priority first
let prioridad_orden = |p: &str| -> u8 {
    match p {
        "urgente" => 0, "alta" => 1, "media" => 2, "baja" => 3, _ => 4,
    }
};
solicitudes.sort_by_key(|s| prioridad_orden(&s.prioridad));
```

### 7. Space-Time Tradeoff

**What:** Trade memory for speed (or vice versa).
**Techniques:**

| Technique | Space Cost | Time Benefit |
|-----------|-----------|--------------|
| HashMap/HashSet precomputation | O(n) | O(n²) → O(n) lookups |
| Prefix sums | O(n) | O(n) → O(1) range queries |
| Memoization cache | O(k) for k unique inputs | Avoids recomputation |
| DB indexes | Disk space | O(n) scan → O(log n) index lookup |
| Denormalized views | Storage duplication | Eliminates joins |

**This is the most commonly applicable paradigm in this project.** Most optimizations in property management code involve building a HashMap or HashSet before a loop.

```rust
// Space-time tradeoff: O(n) memory for O(n·k) → O(n+k) time
let contratos_by_propiedad: HashMap<Uuid, Vec<&contrato::Model>> = contratos
    .iter()
    .fold(HashMap::new(), |mut map, c| {
        map.entry(c.propiedad_id).or_default().push(c);
        map
    });

// Now each property lookup is O(1) instead of O(n)
for propiedad in &propiedades {
    if let Some(matching) = contratos_by_propiedad.get(&propiedad.id) {
        // process matching contracts
    }
}
```

### 8. Backtracking

**What:** Systematic brute force with pruning. Build candidates incrementally, abandon when a constraint is violated.
**Complexity:** Exponential worst-case, but dramatically faster with good pruning.
**Apply when:** Exploring all valid configurations under constraints.

**Pruning strategies:**
- **Constraint propagation:** Eliminate choices that violate known constraints early
- **Symmetry breaking:** Avoid exploring equivalent configurations
- **Feasibility check:** Abandon partial solutions that can't possibly lead to valid complete solutions

### 9. Branch and Bound

**What:** Backtracking + bounding function. Prunes entire subtrees when the bound is worse than the current best known solution.
**Apply when:** Optimization problems where you need the *best* solution, not just any valid one.
**Key components:**
- **Branching:** How to split the problem into subproblems
- **Bounding:** How to estimate the best possible solution from a partial candidate
- **Pruning:** Skip subtrees where the bound is worse than the current best

### 10. Problem Reduction

**What:** Reduce your problem to a well-known problem with a known efficient solution.
**Apply when:** Your problem looks like a variant of a classic.

**Common reductions in property management:**

| Domain Problem | Reduces To | Known Solution |
|---------------|-----------|----------------|
| Contract overlap detection | Interval scheduling | Sort + scan O(n log n) |
| Optimal tenant-unit matching | Bipartite matching | Hungarian algorithm O(n³) |
| Payment allocation across debts | Knapsack variant | DP O(n·W) |
| Maintenance scheduling with dependencies | Topological sort | DFS O(V+E) |
| Shortest route for property inspections | TSP approximation | Nearest neighbor heuristic |

## Amortized Analysis

Amortized analysis studies the average cost of operations over a sequence, rather than the worst-case of a single operation. Three methods:

### Aggregate Method
Sum total cost of n operations, divide by n. Simplest but weakest.

**Example:** `Vec::push` — n pushes cost at most 2n total work (including all reallocations), so amortized cost is O(1) per push.

### Accounting Method
Assign different "charges" to different operations. Cheap operations are overcharged; the surplus pays for expensive ones.

### Potential Method
Define a potential function Φ on the data structure state. Amortized cost = actual cost + ΔΦ. Choose Φ so that expensive operations have large negative ΔΦ.

### Vec::push Amortized Analysis

`Vec::push` is O(1) amortized despite occasional O(n) reallocations. When the buffer is full, `Vec` doubles capacity and copies all elements. Over n pushes, total copies ≤ 2n, giving O(1) per push.

| Pushes | Reallocations | Total Copies |
|--------|---------------|--------------|
| 8 | 3 | 7 |
| 64 | 6 | 63 |
| 1,024 | 10 | 1,023 |
| 1,000,000 | 20 | ~1,000,000 |

**When to use `with_capacity`:** Pre-allocate when you know the final size to eliminate all reallocations:

```rust
// Best: use iterator chain — collects directly with size hint
let pagos_pendientes: Vec<&pago::Model> = pagos
    .iter()
    .filter(|p| p.estado == "pendiente")
    .collect();
```

For collections under ~1,000 elements (typical in property management queries), reallocation overhead is negligible.

## Common Anti-Patterns with Paradigm Mapping

Each anti-pattern is tagged with the paradigm that fixes it.

### Nested `.contains()` Loops — O(n·m) → Space-Time Tradeoff

```rust
// Anti-pattern: O(n·m) — for each contrato, scan entire inquilinos Vec
let cedulas_activas: Vec<String> = inquilinos
    .iter()
    .map(|i| i.cedula.clone())
    .collect();
for contrato in &contratos {
    if cedulas_activas.contains(&contrato_cedula) { /* O(n) scan */ }
}

// Fix (Space-Time Tradeoff): HashSet for O(1) membership testing
let cedulas_activas: HashSet<&str> = inquilinos
    .iter()
    .map(|i| i.cedula.as_str())
    .collect();
for contrato in &contratos {
    if cedulas_activas.contains(contrato_cedula.as_str()) { /* O(1) */ }
}
// Total: O(n + m) instead of O(n·m)
```

**Detection heuristic:** Any `.contains()`, `.iter().any()`, or `.iter().find()` on a `Vec` or slice inside a loop.

### Repeated `.find()` on Unsorted Data — O(n·k) → Space-Time Tradeoff

```rust
// Anti-pattern: O(n·k) — for each contrato, scan all propiedades
for contrato in &contratos {
    let propiedad = propiedades.iter().find(|p| p.id == contrato.propiedad_id);
}

// Fix: build HashMap lookup table once — O(n + k)
let propiedades_por_id: HashMap<Uuid, &propiedad::Model> = propiedades
    .iter()
    .map(|p| (p.id, p))
    .collect();
for contrato in &contratos {
    if let Some(prop) = propiedades_por_id.get(&contrato.propiedad_id) { /* O(1) */ }
}
```

### Unnecessary Sort for Min/Max — O(n log n) → Decrease and Conquer

```rust
// Anti-pattern: O(n log n) sort to find a single element
let mut pagos_sorted = pagos.clone();
pagos_sorted.sort_by_key(|p| p.fecha_vencimiento);
let ultimo_pago = pagos_sorted.last();

// Fix (Decrease and Conquer): O(n) single pass
let ultimo_pago = pagos.iter().max_by_key(|p| p.fecha_vencimiento);
```

**When sorting IS appropriate:**
- You need the top-k elements (sort + truncate, or use `BinaryHeap`)
- You need the full sorted order for display or sequential processing
- You need to detect adjacent duplicates or overlaps (Transform and Conquer)

### Collect-Then-Iterate — Eliminate Unnecessary Work

```rust
// Anti-pattern: allocates intermediate Vec just to sum
let montos: Vec<Decimal> = pagos.iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .collect(); // unnecessary allocation
let total: Decimal = montos.iter().sum();

// Fix: chain directly — no intermediate allocation
let total: Decimal = pagos.iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .sum();
```

**Detection heuristic:** `.collect::<Vec<_>>()` immediately followed by `.iter()` on the result.

### Exponential Recomputation → Dynamic Programming (Memoization)

```rust
// Anti-pattern: same monthly total recomputed for overlapping trend ranges
fn total_mes(pagos: &[pago::Model], anio: i32, mes: u32) -> Decimal {
    pagos.iter()
        .filter(|p| p.fecha_vencimiento.year() == anio
            && p.fecha_vencimiento.month() == mes)
        .map(|p| p.monto)
        .sum()
}
// Called 12+ times with repeated (anio, mes) pairs

// Fix (DP — Memoization): compute each month once
let mut cache: HashMap<(i32, u32), Decimal> = HashMap::new();
let total = *cache.entry((anio, mes)).or_insert_with(|| total_mes(pagos, anio, mes));
```

## Contract Overlap Detection: Algorithmic Alternatives

Detecting overlapping date ranges is a core business rule. Two ranges `[a_start, a_end]` and `[b_start, b_end]` overlap when `a_start <= b_end AND b_start <= a_end`.

### Approach 1: Brute Force — O(n²)

Paradigm: Brute Force. Compare every pair.

```rust
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

Only acceptable for < 20 contracts per property.

### Approach 2: Sort-Then-Scan — O(n log n)

Paradigm: Transform and Conquer (instance simplification).

```rust
fn find_overlaps_sorted(contratos: &[contrato::Model]) -> Vec<(Uuid, Uuid)> {
    let mut por_propiedad: HashMap<Uuid, Vec<&contrato::Model>> = HashMap::new();
    for c in contratos {
        por_propiedad.entry(c.propiedad_id).or_default().push(c);
    }
    let mut overlaps = Vec::new();
    for (_propiedad_id, mut grupo) in por_propiedad {
        grupo.sort_by_key(|c| c.fecha_inicio);
        for window in grupo.windows(2) {
            if window[0].fecha_fin >= window[1].fecha_inicio {
                overlaps.push((window[0].id, window[1].id));
            }
        }
    }
    overlaps
}
```

**Recommended for this project.** Property management typically has < 50 contracts per property.

### Approach 3: Interval Tree — O(n log n) Build, O(log n + k) Query

Paradigm: Transform and Conquer (representation change).

Use when: frequent overlap queries against a large, mostly-static set (> 10,000 contracts). Not needed for this project.

### Comparison Table

| Approach | Paradigm | Build | Query | Space | Best For |
|----------|----------|-------|-------|-------|----------|
| Brute force | Brute Force | — | O(n²) | O(1) | n < 20 |
| Sort-then-scan | Transform & Conquer | O(n log n) | O(n) | O(n) | Single-property validation |
| Interval tree | Transform & Conquer | O(n log n) | O(log n + k) | O(n) | Repeated queries on static data |

## Complexity Cheat Sheet for Domain Operations

| Operation | Naive | Paradigm | Optimized | Improvement |
|-----------|-------|----------|-----------|-------------|
| Find tenant by cedula | `Vec::find` O(n) | Space-Time Tradeoff | `HashMap` O(1) | O(n) → O(1) |
| Check cedula exists | `Vec::contains` O(n) | Space-Time Tradeoff | `HashSet` O(1) | O(n) → O(1) |
| Payments for contract | `filter` O(n) per query | Space-Time Tradeoff | `HashMap<Uuid, Vec>` O(1) | O(n·k) → O(n+k) |
| Contract overlap | Pairwise O(n²) | Transform & Conquer | Sort + scan O(n log n) | O(n²) → O(n log n) |
| Latest payment | Sort + last O(n log n) | Decrease & Conquer | `max_by_key` O(n) | O(n log n) → O(n) |
| Total paid amount | Collect + sum O(n) | Eliminate waste | Direct chain O(n) | Same time, zero alloc |
| Properties by ciudad | Nested loop O(n·m) | Space-Time Tradeoff | `HashMap` group O(1) | O(n·m) → O(n+m) |
| Monthly trend totals | Recompute each O(n×m) | Dynamic Programming | Memoize O(n+m) | O(n×m) → O(n+m) |
| Payment allocation | Try all combos O(2ⁿ) | Greedy | Sort + allocate O(n log n) | Exponential → O(n log n) |
| Maintenance scheduling | Unordered O(n²) | Greedy | Priority sort O(n log n) | O(n²) → O(n log n) |
