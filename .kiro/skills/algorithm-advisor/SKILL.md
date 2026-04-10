---
name: algorithm-advisor
description: >
  Evaluates and fixes algorithm efficiency and data structure selection in Rust code
  by directly editing source files. Replaces O(n²) patterns with O(n log n) or O(n)
  alternatives, swaps suboptimal collections, and optimizes iterator chains. Runs
  cargo fmt and cargo clippy after changes. Use when optimizing algorithms, choosing
  data structures, or fixing quadratic complexity patterns.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: project
  version: "1.0.0"
  domain: algorithms
  triggers: algorithm, data structure, complexity, HashMap, BTreeMap, Vec, O(n)
  role: specialist
  scope: analysis
  output-format: report
  related-skills: perf-optimizer, maintainability-reviewer
---

# Algorithm Advisor

Specialist skill for evaluating and fixing algorithm efficiency and data structure selection in Rust codebases. Actively refactors quadratic patterns, replaces suboptimal collections, and optimizes iterator chains in property management domain operations. Validates changes with cargo fmt and cargo clippy.

## Core Workflow

1. **Identify nested loops replaceable with hash-based lookups** — Find nested `for` loops or `.iter().find()` inside loops over collections where building a `HashMap` or `HashSet` first would reduce O(n²) to O(n)
2. **Evaluate collection type choices** — Check whether `HashMap` vs `BTreeMap`, `Vec` vs `VecDeque`, and `HashSet` vs `BTreeSet` are used appropriately based on access patterns, ordering needs, and collection size
3. **Check for O(n²) patterns in domain operations** — Detect quadratic complexity in contract overlap detection, payment aggregation by contrato, tenant search by cedula, and property filtering operations
4. **Analyze iterator chain efficiency** — Compare iterator chain usage vs manual loop implementations, flag cases where iterator chains would be clearer or more efficient, and detect unnecessary intermediate allocations
5. **Flag unnecessary sorting or repeated linear searches** — Identify repeated `.find()` or `.contains()` on unsorted collections where a single sort-then-binary-search or a `HashSet` lookup would be more efficient

## Reference Guide

Load detailed guidance based on context:

| Topic | Reference | Load When |
|-------|-----------|-----------|
| Data Structures | `references/data-structures.md` | Collection selection, HashMap vs BTreeMap |
| Complexity Analysis | `references/complexity-analysis.md` | Big-O evaluation, amortized analysis |

## Detection Rules

### Nested Loop → Hash Lookup

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

// Fix: O(n) with HashMap lookup
use std::collections::HashMap;
let contratos_by_propiedad: HashMap<i32, Vec<&Contrato>> = contratos
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

### Suboptimal Collection Choice

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

### O(n²) Contract Overlap Detection

Detect quadratic patterns in date range overlap checks common in contract management:

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

// Fix: O(n log n) sort-then-scan approach
let mut sorted = contratos.clone();
sorted.sort_by_key(|c| c.fecha_inicio);
for window in sorted.windows(2) {
    if window[0].fecha_fin >= window[1].fecha_inicio {
        // overlap found between consecutive contracts
    }
}
```

### Repeated Linear Search

Flag repeated `.find()`, `.contains()`, or `.iter().any()` calls on the same collection inside a loop:

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

### Iterator Chain vs Manual Loop

Flag manual loops that could be expressed more efficiently or clearly as iterator chains:

```rust
// Anti-pattern: manual loop with intermediate Vec for payment aggregation
let mut totales: Vec<f64> = Vec::new();
for pago in &pagos {
    if pago.estado == "pagado" {
        totales.push(pago.monto);
    }
}
let total: f64 = totales.iter().sum();

// Fix: iterator chain avoids intermediate allocation
let total: f64 = pagos
    .iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .sum();
```

### Unnecessary Sorting

Flag sorting when only a min/max or top-k is needed:

```rust
// Anti-pattern: O(n log n) sort to find the latest payment
let mut pagos_sorted = pagos.clone();
pagos_sorted.sort_by_key(|p| p.fecha_pago);
let ultimo_pago = pagos_sorted.last();

// Fix: O(n) with iterator max
let ultimo_pago = pagos.iter().max_by_key(|p| p.fecha_pago);
```

## Constraints

### MUST DO
- Directly edit source files to apply algorithm and data structure improvements
- Run `cargo fmt` after changes to maintain formatting
- Run `cargo clippy --all-targets` to validate changes introduce no warnings
- Log each change with file path, before/after Big-O complexity, and rationale
- Use property management domain terminology (contratos, pagos, inquilinos, propiedades)

### MUST NOT DO
- Suggest fixes without applying them — always edit the code directly
- Prematurely optimize collections under 100 elements without noting the tradeoff
- Break idiomatic iterator chains that are already efficient
- Change data structures without considering the full usage pattern (read vs write frequency, ordering needs)
