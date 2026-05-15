---
name: algorithm-advisor
description: >
  Evaluates and fixes algorithm efficiency and data structure selection in Rust code
  by directly editing source files. Applies proven algorithm design paradigms (divide
  and conquer, dynamic programming, greedy, transform and conquer, space-time tradeoffs,
  decrease and conquer) to reduce time complexity. Replaces O(n²) patterns with O(n log n)
  or O(n) alternatives, swaps suboptimal collections, and optimizes iterator chains.
  Runs cargo fmt and cargo clippy after changes. Use when optimizing algorithms, choosing
  data structures, or fixing quadratic complexity patterns.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: project
  version: "2.0.0"
  domain: algorithms
  triggers: algorithm, data structure, complexity, HashMap, BTreeMap, Vec, O(n), optimize, performance, quadratic, time complexity
  role: specialist
  scope: analysis
  output-format: report
  related-skills: perf-optimizer, maintainability-reviewer
---

# Algorithm Advisor

Specialist skill for evaluating and fixing algorithm efficiency and data structure selection in Rust codebases. Uses proven algorithm design paradigms from CLRS, Skiena, and Levitin to systematically reduce time complexity. Actively refactors quadratic patterns, selects optimal design strategies, replaces suboptimal collections, and optimizes iterator chains in property management domain operations. Validates changes with cargo fmt and cargo clippy.

## Systematic Optimization Workflow

Follow this structured method for every optimization task. Do not skip steps.

### Step 1: Profile First, Optimize Second

Never guess where the bottleneck is. Measure before changing anything.

- Use `tracing` spans with timing to identify slow code paths
- Check `cargo bench` (criterion) results if benchmarks exist
- Identify the dominant operation: "What grows when the dataset grows?"
- In this project, *n* is usually the number of DB rows returned or entities being joined

### Step 2: Classify the Current Complexity

Map the code to its Big-O. Common patterns to spot:

- Nested loops over the same collection → O(n²)
- Loop with a lookup inside → O(n) × O(lookup cost)
- Sequential independent passes → O(n) + O(m) = O(n + m) — this is fine
- Sort then scan → O(n log n)
- Recursive with overlapping subproblems → exponential without memoization

### Step 3: Select the Right Design Paradigm

Match the problem structure to a proven paradigm. Try in order of simplicity:

1. **Space-Time Tradeoff** — Can we precompute a lookup table? (HashMap/HashSet)
2. **Greedy** — Does a locally optimal choice at each step yield the global optimum?
3. **Transform and Conquer** — Can we restructure the data first (sort, index, change representation)?
4. **Divide and Conquer** — Can we split into independent subproblems and combine?
5. **Dynamic Programming** — Are there overlapping subproblems with optimal substructure?
6. **Decrease and Conquer** — Can we reduce the problem by a constant or factor each step?

Only go more complex if simpler paradigms don't yield correct or efficient solutions.

### Step 4: Apply the Reduction

Use the paradigm-specific techniques from the reference guides. Log each change with:
- File path
- Before/after Big-O complexity
- Which paradigm was applied
- Rationale for the tradeoff

### Step 5: Validate the Tradeoff

Before committing to an optimization, assess:
- Is *n* actually large enough to matter? (Collections under ~100 elements rarely need optimization)
- Does the optimization increase space complexity unacceptably?
- Does it make the code harder to maintain?
- Does it introduce correctness risks?

### Step 6: Verify with Tooling

- Run `cargo fmt` after changes
- Run `cargo clippy --all-targets` to validate no new warnings
- Run relevant tests to confirm correctness
- Benchmark before and after with real-scale data if possible

## Algorithm Design Paradigms

These are the 10 proven paradigms from foundational CS literature (CLRS, Skiena, Levitin). Each includes when to apply it and Rust-specific examples from the property management domain.

### 1. Brute Force

Try all possibilities. This is the baseline — every optimization is measured against it.

**Complexity:** Usually O(n²), O(2ⁿ), or O(n!)
**When it's acceptable:** Small datasets (< 20 elements), one-off validation, prototyping
**When to replace:** Any loop over > 100 elements with nested search or pairwise comparison

```rust
// Brute force: O(n²) pairwise overlap check — acceptable for < 20 contracts
for i in 0..contratos.len() {
    for j in (i + 1)..contratos.len() {
        if overlaps(&contratos[i], &contratos[j]) {
            // found overlap
        }
    }
}
```

### 2. Decrease and Conquer

Reduce the problem by a constant amount (usually 1) or constant factor (usually half) each step.

**Complexity:** O(n) for decrease-by-one, O(log n) for decrease-by-half
**Apply when:** The solution to a smaller instance directly extends to the full problem
**Domain examples:** Binary search on sorted contracts, insertion sort for nearly-sorted small lists

```rust
// Decrease by half: binary search for a payment due date in sorted list
// O(log n) instead of O(n) linear scan
let idx = pagos_sorted
    .binary_search_by_key(&target_date, |p| p.fecha_vencimiento);
```

### 3. Divide and Conquer

Split into *independent* subproblems, solve recursively, combine results.

**Complexity:** Determined by the Master Theorem — typically O(n log n)
**Apply when:** The problem naturally splits into independent halves
**Domain examples:** Merge sort for report ordering, parallel aggregation of property groups

```rust
// Divide and conquer: process properties in parallel groups
// Split by ciudad, aggregate independently, combine results
let mut por_ciudad: HashMap<&str, Vec<&propiedad::Model>> = HashMap::new();
for p in &propiedades {
    por_ciudad.entry(p.ciudad.as_str()).or_default().push(p);
}
// Each group is processed independently — no cross-group dependencies
for (ciudad, grupo) in &por_ciudad {
    let stats = compute_stats(grupo); // independent subproblem
    resultados.push((ciudad, stats));
}
```

### 4. Transform and Conquer

Restructure the problem or data *before* solving. Three forms:

**a) Instance simplification** — Sort first, then solve in O(n) instead of O(n²):

```rust
// Transform: sort contracts by fecha_inicio, then scan adjacent pairs
// O(n log n) sort + O(n) scan instead of O(n²) pairwise comparison
let mut sorted = contratos.clone();
sorted.sort_by_key(|c| c.fecha_inicio);
for window in sorted.windows(2) {
    if window[0].fecha_fin >= window[1].fecha_inicio {
        // overlap detected
    }
}
```

**b) Representation change** — Switch data structures:

```rust
// Transform: Vec → HashMap for O(1) lookups instead of O(n) scans
let propiedades_por_id: HashMap<Uuid, &propiedad::Model> = propiedades
    .iter()
    .map(|p| (p.id, p))
    .collect();
```

**c) Problem reduction** — Reduce to a known solved problem:

```rust
// Reduce "find all overlapping contracts" to "find intersecting intervals"
// Use the interval scheduling / sweep line algorithm
```

### 5. Dynamic Programming

For problems with *overlapping subproblems* and *optimal substructure*. Store intermediate results to avoid recomputation.

**Complexity:** Reduces exponential to polynomial
**Apply when:** You see the same subproblem computed multiple times
**Two approaches:** Memoization (top-down, lazy) and Tabulation (bottom-up, eager)

```rust
// DP with memoization: compute cumulative rent for date ranges
// Without DP: recalculates overlapping month ranges repeatedly
// With DP: each month's total is computed once and reused
use std::collections::HashMap;

fn ingresos_acumulados(
    pagos: &[pago::Model],
    meses: &[(i32, u32)], // (year, month) pairs
    cache: &mut HashMap<(i32, u32), Decimal>,
) -> Vec<Decimal> {
    meses.iter().map(|&(anio, mes)| {
        *cache.entry((anio, mes)).or_insert_with(|| {
            pagos.iter()
                .filter(|p| p.fecha_vencimiento.year() == anio
                    && p.fecha_vencimiento.month() == mes
                    && p.estado == "pagado")
                .map(|p| p.monto)
                .sum()
        })
    }).collect()
}
```

### 6. Greedy

Make the locally optimal choice at each step. Only works when the greedy choice property holds.

**Complexity:** Usually O(n log n) due to initial sort, O(n) for the greedy pass
**Apply when:** You can prove that local optimum leads to global optimum
**Domain examples:** Scheduling maintenance by priority, allocating payments to oldest debts first

```rust
// Greedy: allocate a payment to the oldest pending debts first
// Sort debts by fecha_vencimiento (oldest first), apply payment greedily
let mut deudas_sorted = deudas_pendientes.clone();
deudas_sorted.sort_by_key(|d| d.fecha_vencimiento);

let mut monto_restante = pago.monto;
for deuda in &mut deudas_sorted {
    if monto_restante <= Decimal::ZERO { break; }
    let aplicar = monto_restante.min(deuda.monto_pendiente);
    deuda.monto_pendiente -= aplicar;
    monto_restante -= aplicar;
}
```

### 7. Space-Time Tradeoff

Trade memory for speed. The most commonly applicable paradigm in this project.

**Techniques:**
- **Precomputation:** Lookup tables, prefix sums
- **Hashing:** O(n²) search → O(n) with HashSet/HashMap
- **Caching/Memoization:** Store expensive results for reuse
- **Indexing:** DB indexes for frequently filtered columns

```rust
// Space-time tradeoff: build HashMap once, O(1) lookups thereafter
// Trades O(n) memory for O(n·k) → O(n+k) time improvement
let contratos_by_propiedad: HashMap<Uuid, Vec<&contrato::Model>> = contratos
    .iter()
    .fold(HashMap::new(), |mut map, c| {
        map.entry(c.propiedad_id).or_default().push(c);
        map
    });
```

### 8. Backtracking

Systematic brute force with pruning. Build candidates incrementally, abandon when a constraint is violated.

**Complexity:** Exponential worst-case, but dramatically faster in practice with good pruning
**Apply when:** Exploring all valid configurations under constraints
**Domain examples:** Finding valid contract assignment combinations, scheduling with constraints

```rust
// Backtracking: find a valid assignment of tenants to units
// Prune early when a constraint is violated (e.g., unit already occupied)
fn assign_tenants(
    units: &[unidad::Model],
    tenants: &[inquilino::Model],
    assignment: &mut Vec<(Uuid, Uuid)>,
    used_units: &mut HashSet<Uuid>,
    idx: usize,
) -> bool {
    if idx == tenants.len() { return true; } // all assigned
    for unit in units {
        if used_units.contains(&unit.id) { continue; } // prune
        used_units.insert(unit.id);
        assignment.push((unit.id, tenants[idx].id));
        if assign_tenants(units, tenants, assignment, used_units, idx + 1) {
            return true;
        }
        assignment.pop(); // backtrack
        used_units.remove(&unit.id);
    }
    false
}
```

### 9. Branch and Bound

Backtracking + bounding function that estimates the best possible solution from a partial candidate. Prunes entire subtrees when the bound is worse than the current best.

**Complexity:** Exponential worst-case, but much better with tight bounds
**Apply when:** Optimization problems where you need the best solution, not just any valid one
**Domain examples:** Optimal expense allocation, minimizing vacancy periods

### 10. Problem Reduction

Reduce your problem to a well-known problem with a known efficient solution. If you can reduce in O(n), you inherit the known solution's complexity.

**Apply when:** Your problem looks like a variant of a classic (shortest path, matching, scheduling, knapsack)
**Domain examples:**
- Contract overlap detection → interval scheduling problem
- Optimal tenant-unit matching → bipartite matching
- Payment allocation → knapsack variant

## Core Detection Rules

### Nested Loop → Hash Lookup (Space-Time Tradeoff)

Look for nested iterations where the inner loop searches a collection. Building a lookup table first reduces O(n²) to O(n).

```rust
// Anti-pattern: O(n²) nested search for contratos matching propiedades
for propiedad in &propiedades {
    for contrato in &contratos {
        if contrato.propiedad_id == propiedad.id {
            // process match
        }
    }
}

// Fix: O(n) with HashMap lookup — Space-Time Tradeoff paradigm
use std::collections::HashMap;
let contratos_by_propiedad: HashMap<Uuid, Vec<&Contrato>> = contratos
    .iter()
    .fold(HashMap::new(), |mut map, c| {
        map.entry(c.propiedad_id).or_default().push(c);
        map
    });
for propiedad in &propiedades {
    if let Some(matching) = contratos_by_propiedad.get(&propiedad.id) {
        for contrato in matching {
            // process match
        }
    }
}
```

### Suboptimal Collection Choice (Transform and Conquer — Representation Change)

Flag cases where the wrong collection type is used for the access pattern:

```rust
// Anti-pattern: BTreeMap when order doesn't matter for payment lookup
let pagos_por_contrato: BTreeMap<i32, Vec<Pago>> = /* ... */;
// Only accessed via .get(&contrato_id) — ordering is unused

// Fix: HashMap for O(1) average lookup when ordering is not needed
let pagos_por_contrato: HashMap<i32, Vec<Pago>> = /* ... */;
```

```rust
// Anti-pattern: Vec with frequent front insertion for a processing queue
let mut cola_pagos: Vec<Pago> = Vec::new();
cola_pagos.insert(0, nuevo_pago); // O(n) shift on every insert

// Fix: VecDeque for O(1) front insertion
use std::collections::VecDeque;
let mut cola_pagos: VecDeque<Pago> = VecDeque::new();
cola_pagos.push_front(nuevo_pago);
```

### O(n²) Contract Overlap → Sort-Then-Scan (Transform and Conquer — Instance Simplification)

```rust
// Anti-pattern: O(n²) pairwise overlap check
for i in 0..contratos.len() {
    for j in (i + 1)..contratos.len() {
        if contratos[i].fecha_fin >= contratos[j].fecha_inicio
            && contratos[j].fecha_fin >= contratos[i].fecha_inicio
        {
            // overlap found
        }
    }
}

// Fix: O(n log n) sort-then-scan — Transform and Conquer
let mut sorted = contratos.clone();
sorted.sort_by_key(|c| c.fecha_inicio);
for window in sorted.windows(2) {
    if window[0].fecha_fin >= window[1].fecha_inicio {
        // overlap found between consecutive contracts
    }
}
```

### Repeated Linear Search → Precomputed Lookup (Space-Time Tradeoff)

```rust
// Anti-pattern: O(n*m) repeated linear search for tenant cedulas
let cedulas_activas: Vec<String> = inquilinos
    .iter()
    .map(|i| i.cedula.clone())
    .collect();

for contrato in &contratos {
    if cedulas_activas.contains(&contrato.cedula_inquilino) {
        // process active tenant contract
    }
}

// Fix: O(n+m) with HashSet for membership testing
use std::collections::HashSet;
let cedulas_activas: HashSet<&str> = inquilinos
    .iter()
    .map(|i| i.cedula.as_str())
    .collect();

for contrato in &contratos {
    if cedulas_activas.contains(contrato.cedula_inquilino.as_str()) {
        // process active tenant contract
    }
}
```

### Unnecessary Sorting → Iterator Min/Max (Decrease and Conquer)

```rust
// Anti-pattern: O(n log n) sort to find the latest payment
let mut pagos_sorted = pagos.clone();
pagos_sorted.sort_by_key(|p| p.fecha_pago);
let ultimo_pago = pagos_sorted.last();

// Fix: O(n) with iterator max — single pass, no allocation
let ultimo_pago = pagos.iter().max_by_key(|p| p.fecha_pago);
```

### Collect-Then-Iterate → Direct Chain (Eliminate Unnecessary Work)

```rust
// Anti-pattern: intermediate allocation just to sum
let montos: Vec<Decimal> = pagos
    .iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .collect(); // unnecessary allocation
let total: Decimal = montos.iter().sum();

// Fix: chain directly — no intermediate allocation
let total: Decimal = pagos
    .iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .sum();
```

### Exponential Recomputation → Memoization (Dynamic Programming)

```rust
// Anti-pattern: recomputing monthly totals for overlapping date ranges
fn total_mes(pagos: &[pago::Model], anio: i32, mes: u32) -> Decimal {
    pagos.iter()
        .filter(|p| p.fecha_vencimiento.year() == anio
            && p.fecha_vencimiento.month() == mes
            && p.estado == "pagado")
        .map(|p| p.monto)
        .sum()
}
// Called repeatedly for the same (anio, mes) in trend calculations

// Fix: memoize with HashMap cache
let mut cache: HashMap<(i32, u32), Decimal> = HashMap::new();
let total = *cache.entry((anio, mes)).or_insert_with(|| {
    pagos.iter()
        .filter(|p| p.fecha_vencimiento.year() == anio
            && p.fecha_vencimiento.month() == mes
            && p.estado == "pagado")
        .map(|p| p.monto)
        .sum()
});
```

## Paradigm Selection Quick Reference

| Problem Pattern | Paradigm | Typical Reduction |
|----------------|----------|-------------------|
| Nested loop with inner search | Space-Time Tradeoff (HashMap) | O(n²) → O(n) |
| Pairwise comparison | Transform & Conquer (sort first) | O(n²) → O(n log n) |
| Repeated computation of same values | Dynamic Programming (memoize) | O(2ⁿ) → O(n) |
| Find min/max/top-k | Decrease & Conquer (single pass) | O(n log n) → O(n) |
| Locally optimal choices build global solution | Greedy | O(n²) → O(n log n) |
| Independent subproblems combinable | Divide & Conquer | O(n²) → O(n log n) |
| Wrong data structure for access pattern | Transform & Conquer (representation) | Varies |
| Constraint satisfaction / enumeration | Backtracking with pruning | Exponential → practical |
| Optimization with bounds | Branch & Bound | Exponential → practical |
| Looks like a known classic problem | Problem Reduction | Inherit known solution |

## Reference Guide

Load detailed guidance based on context:

| Topic | Reference | Load When |
|-------|-----------|-----------|
| Data Structures | `references/data-structures.md` | Collection selection, HashMap vs BTreeMap |
| Complexity Analysis | `references/complexity-analysis.md` | Big-O evaluation, amortized analysis, paradigm details |

## Constraints

### MUST DO
- Follow the 6-step systematic workflow for every optimization task
- Identify which paradigm applies before writing code
- Directly edit source files to apply algorithm and data structure improvements
- Run `cargo fmt` after changes to maintain formatting
- Run `cargo clippy --all-targets` to validate changes introduce no warnings
- Log each change with file path, before/after Big-O complexity, paradigm used, and rationale
- Use property management domain terminology (contratos, pagos, inquilinos, propiedades)
- Assess whether *n* is large enough to justify the optimization

### MUST NOT DO
- Suggest fixes without applying them — always edit the code directly
- Skip the paradigm selection step — always identify which paradigm applies
- Prematurely optimize collections under 100 elements without noting the tradeoff
- Break idiomatic iterator chains that are already efficient
- Change data structures without considering the full usage pattern (read vs write frequency, ordering needs)
- Apply complex paradigms (DP, backtracking) when simpler ones (greedy, space-time tradeoff) suffice
