# Performance Analysis: `enrich_contratos` Endpoint

## Verdict: Do NOT Benchmark the Rust Code

Optimizing iterators or reducing allocations in the mapping logic would be **the wrong optimization target**. The bottleneck is IO-bound, not CPU-bound.

## Root Cause: Sequential DB Round-Trips

The endpoint makes **60 sequential queries** (20 contracts × 3 queries each). With ~3-5ms per round-trip on the same k8s cluster:

- 60 queries × ~5ms avg = **~300ms** (matches observed p50)

The Rust code between queries (struct construction, a single `Vec::push`) takes **nanoseconds**. Even if you eliminated every allocation entirely, you'd save <1μs total — invisible against 300ms of network IO.

## Why Benchmarking Won't Help

A microbenchmark of the mapping logic would measure:
- One `Vec::with_capacity(20)` allocation (~50ns)
- 20 struct moves into the vec (~200ns)
- Total CPU work: **<1μs**

This is 0.0003% of the endpoint latency. No iterator trick or allocation optimization will make a measurable difference.

## What Would Actually Fix It

The fix is reducing DB round-trips. Two approaches, in order of impact:

### Option A: Batch Queries (Recommended — simplest, biggest win)

Replace 60 individual queries with 3 batch queries using `WHERE id = ANY($1)`:

```rust
pub async fn enrich_contratos(
    db: &DatabasePool,
    contratos: Vec<Contrato>,
) -> Result<Vec<ContratoDetalle>, AppError> {
    let propiedad_ids: Vec<Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let inquilino_ids: Vec<Uuid> = contratos.iter().map(|c| c.inquilino_id).collect();
    let contrato_ids: Vec<Uuid> = contratos.iter().map(|c| c.id).collect();

    // 3 queries total instead of 60
    let (propiedades, inquilinos, pagos_stats) = tokio::try_join!(
        db.query("SELECT id, titulo FROM propiedades WHERE id = ANY($1)", &[&propiedad_ids]),
        db.query("SELECT id, nombre || ' ' || apellido as nombre_completo FROM inquilinos WHERE id = ANY($1)", &[&inquilino_ids]),
        db.query(
            "SELECT contrato_id, \
             COUNT(*) FILTER (WHERE estado = 'pendiente') as pendientes, \
             MAX(fecha_pago) as ultimo_pago \
             FROM pagos WHERE contrato_id = ANY($1) GROUP BY contrato_id",
            &[&contrato_ids]
        ),
    )?;

    // Build lookup maps
    let prop_map: HashMap<Uuid, String> = propiedades.into_iter()
        .map(|r| (r.get("id"), r.get("titulo")))
        .collect();
    let inq_map: HashMap<Uuid, String> = inquilinos.into_iter()
        .map(|r| (r.get("id"), r.get("nombre_completo")))
        .collect();
    let pago_map: HashMap<Uuid, (i64, Option<NaiveDate>)> = pagos_stats.into_iter()
        .map(|r| (r.get("contrato_id"), (r.get("pendientes"), r.get("ultimo_pago"))))
        .collect();

    // Map results — this is the only CPU work, and it's trivial
    let detalles = contratos.into_iter().map(|contrato| {
        ContratoDetalle {
            propiedad_nombre: prop_map.get(&contrato.propiedad_id).cloned().unwrap_or_default(),
            inquilino_nombre: inq_map.get(&contrato.inquilino_id).cloned().unwrap_or_default(),
            pagos_pendientes: pago_map.get(&contrato.id).map(|(p, _)| *p).unwrap_or(0),
            ultimo_pago: pago_map.get(&contrato.id).and_then(|(_, u)| *u),
            contrato,
        }
    }).collect();

    Ok(detalles)
}
```

**Expected improvement**: 60 round-trips → 3 concurrent round-trips. Latency drops from ~300ms to ~5-8ms (single slowest query time).

### Option B: Single JOIN Query (Maximum efficiency)

```sql
SELECT c.*, p.titulo as propiedad_nombre,
       i.nombre || ' ' || i.apellido as inquilino_nombre,
       COALESCE(pago.pendientes, 0) as pagos_pendientes,
       pago.ultimo_pago
FROM contratos c
JOIN propiedades p ON p.id = c.propiedad_id
JOIN inquilinos i ON i.id = c.inquilino_id
LEFT JOIN LATERAL (
    SELECT COUNT(*) FILTER (WHERE estado = 'pendiente') as pendientes,
           MAX(fecha_pago) as ultimo_pago
    FROM pagos WHERE contrato_id = c.id
) pago ON true
WHERE c.id = ANY($1)
```

**Expected improvement**: 60 round-trips → 1 query. Latency drops to ~3-5ms.

## Summary

| Approach | Queries | Expected Latency | Effort |
|----------|---------|-----------------|--------|
| Current (sequential) | 60 | ~300ms | — |
| Batch + try_join! | 3 (concurrent) | ~5-8ms | Low |
| Single JOIN | 1 | ~3-5ms | Medium |
| "Optimize Rust iterators" | 60 | ~299.999ms | Wasted |

## Recommendation

Go with **Option A** (batch queries + `try_join!`). It's the simplest change, gives a ~40x speedup, and keeps the code readable. The Rust mapping code is already fine — `Vec::with_capacity` is used, the struct construction is minimal, and there's nothing to optimize on the CPU side.

Do not spend time benchmarking the Rust code. The answer is architectural, not algorithmic.
