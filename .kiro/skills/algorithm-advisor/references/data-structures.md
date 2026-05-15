# Data Structures Reference Guide

## HashMap vs BTreeMap

### When to Use HashMap

`HashMap<K, V>` provides O(1) average-case lookup, insertion, and removal. Use it when you need fast key-based access and don't care about iteration order.

**Domain example — payment lookup by contrato_id:**

```rust
use std::collections::HashMap;
use uuid::Uuid;

// Build a lookup table for payments grouped by contract
let pagos_por_contrato: HashMap<Uuid, Vec<&pago::Model>> = pagos
    .iter()
    .fold(HashMap::new(), |mut map, p| {
        map.entry(p.contrato_id).or_default().push(p);
        map
    });

// O(1) lookup per contract
if let Some(pagos_contrato) = pagos_por_contrato.get(&contrato.id) {
    let total: Decimal = pagos_contrato
        .iter()
        .filter(|p| p.estado == "pagado")
        .map(|p| p.monto)
        .sum();
}
```

### When to Use BTreeMap

`BTreeMap<K, V>` provides O(log n) operations but maintains keys in sorted order. Use it when you need ordered iteration or range queries.

**Domain example — contract date range storage:**

```rust
use std::collections::BTreeMap;
use sea_orm::entity::prelude::Date;

// Store contracts indexed by start date for chronological access
let contratos_por_fecha: BTreeMap<Date, Vec<&contrato::Model>> = contratos
    .iter()
    .fold(BTreeMap::new(), |mut map, c| {
        map.entry(c.fecha_inicio).or_default().push(c);
        map
    });

// Iterate in chronological order — BTreeMap guarantees sorted keys
for (fecha, contratos_en_fecha) in &contratos_por_fecha {
    // process contracts starting on this date
}

// Range query: contracts starting in Q1 2025
let q1_start = Date::from_ymd_opt(2025, 1, 1).unwrap();
let q1_end = Date::from_ymd_opt(2025, 4, 1).unwrap();
for (fecha, contratos_en_fecha) in contratos_por_fecha.range(q1_start..q1_end) {
    // process Q1 contracts
}
```

### Selection Criteria

| Criterion | HashMap | BTreeMap |
|-----------|---------|----------|
| Lookup speed | O(1) average | O(log n) |
| Insertion speed | O(1) amortized | O(log n) |
| Ordered iteration | No | Yes (by key) |
| Range queries | No | Yes (`.range()`) |
| Memory overhead | Higher (hash table) | Lower (B-tree nodes) |
| Key requirement | `Hash + Eq` | `Ord` |
| Small collections (<50) | Either — difference negligible | Slightly better cache locality |

**Rule of thumb:** Default to `HashMap`. Switch to `BTreeMap` when you need sorted iteration, range queries, or deterministic ordering (e.g., for reproducible report output).


## Vec vs VecDeque

### When to Use Vec

`Vec<T>` is the default sequential collection. It provides O(1) amortized push/pop at the back and O(1) random access. Use it for most ordered collections.

```rust
// Standard pattern: collect SeaORM results into a Vec for response
let propiedades: Vec<propiedad::Model> = propiedad::Entity::find()
    .all(db)
    .await?;

let responses: Vec<PropiedadResponse> = propiedades
    .into_iter()
    .map(PropiedadResponse::from)
    .collect();
```

### When to Use VecDeque

`VecDeque<T>` is a double-ended queue backed by a ring buffer. It provides O(1) push/pop at both front and back. Use it when you need efficient front insertion or a FIFO queue.

**Domain example — payment processing queue:**

```rust
use std::collections::VecDeque;

// Processing queue where new urgent payments go to the front
let mut cola_pagos: VecDeque<pago::Model> = pagos_pendientes.into();

// O(1) front insertion for priority payments
cola_pagos.push_front(pago_urgente);

// O(1) back insertion for normal payments
cola_pagos.push_back(pago_normal);

// Process from front
while let Some(pago) = cola_pagos.pop_front() {
    process_pago(&pago).await?;
}
```

**Anti-pattern — Vec with front insertion:**

```rust
// Bad: O(n) shift on every insert at position 0
let mut cola_pagos: Vec<pago::Model> = Vec::new();
cola_pagos.insert(0, pago_urgente); // shifts all elements right

// Fix: use VecDeque
let mut cola_pagos: VecDeque<pago::Model> = VecDeque::new();
cola_pagos.push_front(pago_urgente); // O(1)
```

### Selection Criteria

| Criterion | Vec | VecDeque |
|-----------|-----|----------|
| Push/pop back | O(1) amortized | O(1) amortized |
| Push/pop front | O(n) — shifts all elements | O(1) |
| Random access | O(1), contiguous memory | O(1), but not contiguous |
| Cache friendliness | Excellent (contiguous) | Good (ring buffer, two segments) |
| Slice access | `.as_slice()` — single slice | `.make_contiguous()` needed first |
| Default choice | Yes | Only when front ops are needed |

**Rule of thumb:** Default to `Vec`. Switch to `VecDeque` only when you need O(1) front insertion/removal (queues, sliding windows).

## HashSet vs BTreeSet

### When to Use HashSet

`HashSet<T>` provides O(1) average membership testing. Use it for deduplication and fast `contains()` checks.

**Domain example — tenant search by cedula:**

```rust
use std::collections::HashSet;

// Build a set of active tenant cedulas for fast membership checks
let cedulas_activas: HashSet<&str> = inquilinos
    .iter()
    .filter(|i| /* tenant has active contract */)
    .map(|i| i.cedula.as_str())
    .collect();

// O(1) per check instead of O(n) linear scan
for contrato in &contratos {
    let inquilino = inquilinos.iter().find(|i| i.id == contrato.inquilino_id);
    if let Some(inq) = inquilino {
        if cedulas_activas.contains(inq.cedula.as_str()) {
            // tenant is active
        }
    }
}
```

**Anti-pattern — Vec::contains for repeated membership checks:**

```rust
// Bad: O(n) per contains call, O(n*m) total
let cedulas_activas: Vec<String> = inquilinos
    .iter()
    .map(|i| i.cedula.clone())
    .collect();

for contrato in &contratos {
    if cedulas_activas.contains(&contrato_cedula) {
        // O(n) scan each time
    }
}

// Fix: HashSet for O(1) lookups, O(n+m) total
let cedulas_activas: HashSet<&str> = inquilinos
    .iter()
    .map(|i| i.cedula.as_str())
    .collect();
```

### When to Use BTreeSet

`BTreeSet<T>` maintains elements in sorted order with O(log n) operations. Use it when you need sorted unique values or range queries.

```rust
use std::collections::BTreeSet;

// Collect unique sorted cities for a dropdown filter
let ciudades: BTreeSet<&str> = propiedades
    .iter()
    .map(|p| p.ciudad.as_str())
    .collect();

// Iteration yields cities in alphabetical order
for ciudad in &ciudades {
    // "La Romana", "Punta Cana", "Santiago", "Santo Domingo"
}
```

### Selection Criteria

| Criterion | HashSet | BTreeSet |
|-----------|---------|----------|
| Contains check | O(1) average | O(log n) |
| Insertion | O(1) amortized | O(log n) |
| Ordered iteration | No | Yes |
| Range queries | No | Yes (`.range()`) |
| Element requirement | `Hash + Eq` | `Ord` |

**Rule of thumb:** Default to `HashSet` for membership testing. Use `BTreeSet` when you need sorted output or range operations.


## SmallVec for Small Collections

### When to Use SmallVec

`SmallVec<[T; N]>` from the `smallvec` crate stores up to N elements inline on the stack, spilling to the heap only when the collection grows beyond N. This eliminates heap allocation for the common case of small collections.

**Domain example — a property's active contracts (typically 0–2):**

```rust
use smallvec::SmallVec;

// Most properties have 0–2 active contracts at any time
// SmallVec<[Uuid; 2]> avoids heap allocation for the common case
fn active_contract_ids(
    contratos: &[contrato::Model],
    propiedad_id: Uuid,
) -> SmallVec<[Uuid; 2]> {
    contratos
        .iter()
        .filter(|c| c.propiedad_id == propiedad_id && c.estado == "activo")
        .map(|c| c.id)
        .collect()
}
```

**Domain example — payment methods per contract (typically 1–3):**

```rust
use smallvec::SmallVec;

// Collect distinct payment methods for a contract — rarely more than 3
fn metodos_pago_usados(pagos: &[pago::Model]) -> SmallVec<[&str; 4]> {
    let mut metodos = SmallVec::<[&str; 4]>::new();
    for pago in pagos {
        if let Some(ref metodo) = pago.metodo_pago {
            if !metodos.contains(&metodo.as_str()) {
                metodos.push(metodo.as_str());
            }
        }
    }
    metodos
}
```

### When NOT to Use SmallVec

- Collections that regularly exceed the inline capacity — the spill to heap negates the benefit and adds overhead from the capacity check.
- Hot paths where the branch prediction cost of checking inline vs heap matters more than the allocation saved.
- When `Vec` with `with_capacity` already avoids reallocations.

### Sizing the Inline Buffer

Choose N based on the expected common-case size:

| Domain Scenario | Expected Size | Suggested N |
|----------------|---------------|-------------|
| Active contracts per property | 0–2 | 2 |
| Payment methods per contract | 1–3 | 4 |
| Validation errors per request | 0–5 | 4 |
| Related entities in a join | 1–10 | 8 |

**Rule of thumb:** Set N to cover 90%+ of real-world cases. If unsure, profile first — a `Vec` with `with_capacity` is often good enough.

## Quick Reference: Collection Selection

| Need | Collection | Why |
|------|-----------|-----|
| Fast key→value lookup | `HashMap` | O(1) average lookup |
| Sorted key→value with range queries | `BTreeMap` | O(log n) with `.range()` |
| Ordered sequence, append-heavy | `Vec` | O(1) push, contiguous memory |
| Double-ended queue, FIFO processing | `VecDeque` | O(1) push/pop both ends |
| Fast membership testing | `HashSet` | O(1) average `contains` |
| Sorted unique values | `BTreeSet` | O(log n), ordered iteration |
| Small fixed-size collections | `SmallVec<[T; N]>` | Stack allocation up to N |

### Domain-Specific Recommendations

| Operation | Recommended Collection | Rationale |
|-----------|----------------------|-----------|
| Payment lookup by `contrato_id` | `HashMap<Uuid, Vec<pago::Model>>` | O(1) lookup by contract, no ordering needed |
| Contract date range storage | `BTreeMap<Date, Vec<contrato::Model>>` | Sorted by date, supports range queries for overlap detection |
| Tenant search by `cedula` | `HashSet<&str>` for membership, `HashMap<&str, &inquilino::Model>` for retrieval | O(1) lookup by cedula |
| Property filtering by `ciudad` | `HashMap<&str, Vec<&propiedad::Model>>` | Group by city for fast filtered access |
| Unique cities/provincias for dropdowns | `BTreeSet<&str>` | Sorted alphabetical output |
| Active contracts per property | `SmallVec<[Uuid; 2]>` | Rarely more than 2, avoids heap allocation |
| Payment processing queue | `VecDeque<pago::Model>` | FIFO with priority insertion at front |
