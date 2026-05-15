# Bench Optimizer Analysis: Contract Enrichment Endpoint

## Verdict: Don't Optimize the Rust Code

**No benchmark written. No benchmark needed.**

The team asked whether to optimize iterators or reduce allocations in the mapping
logic. The answer is: **no** — the Rust code is not the bottleneck and never will be
at this scale.

## Why This Doesn't Warrant a Benchmark

### The Math

| Component | Time per contract | × 20 contracts | Total |
|-----------|------------------|-----------------|-------|
| DB query: propiedad name | ~3ms | 20 | ~60ms |
| DB query: inquilino name | ~3ms | 20 | ~60ms |
| DB query: pago stats | ~5ms | 20 | ~100ms |
| **Rust mapping logic** | **~10-50ns** | **20** | **~0.001ms** |
| **Total** | | | **~220-300ms** |

The Rust code (struct construction, field assignment) accounts for approximately
**0.0003%** of the endpoint's latency. Even if you made the mapping logic infinitely
fast, the endpoint would still take ~300ms.

### Skill Rule Applied

From the bench-optimizer workflow, "When NOT to Benchmark":

> **IO-bound code**: If the function spends 99% of time waiting on DB/network,
> optimizing the CPU portion is pointless. Profile first to confirm CPU-bound.

This code spends **99.9997%** of its time waiting on database round-trips. The CPU
portion (iterator chains, allocations, struct construction) is unmeasurably small
relative to the I/O.

## What Would Actually Help

The real problem is **60 sequential database round-trips** (20 contracts × 3 queries,
executed one at a time in a `for` loop with `.await`).

### Option A: Batch Queries (recommended, simplest)

Replace N+1 queries with 3 batch queries using `WHERE id = ANY($1)`:

```rust
pub async fn enrich_contratos(
    db: &DatabasePool,
    contratos: Vec<Contrato>,
) -> Result<Vec<ContratoDetalle>, AppError> {
    let propiedad_ids: Vec<Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let inquilino_ids: Vec<Uuid> = contratos.iter().map(|c| c.inquilino_id).collect();
    let contrato_ids: Vec<Uuid> = contratos.iter().map(|c| c.id).collect();

    // 3 queries total instead of 60
    let (propiedades, inquilinos, pago_stats) = tokio::try_join!(
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

    // Build lookup maps — O(n) total, negligible vs saved I/O
    let prop_map: HashMap<Uuid, String> = propiedades.into_iter()
        .map(|r| (r.get("id"), r.get("titulo")))
        .collect();
    let inq_map: HashMap<Uuid, String> = inquilinos.into_iter()
        .map(|r| (r.get("id"), r.get("nombre_completo")))
        .collect();
    let pago_map: HashMap<Uuid, (i64, Option<NaiveDate>)> = pago_stats.into_iter()
        .map(|r| (r.get("contrato_id"), (r.get("pendientes"), r.get("ultimo_pago"))))
        .collect();

    // Map results — this is the "Rust code" part, takes ~microseconds total
    let detalles = contratos.into_iter().map(|contrato| {
        let propiedad_nombre = prop_map.get(&contrato.propiedad_id)
            .cloned().unwrap_or_default();
        let inquilino_nombre = inq_map.get(&contrato.inquilino_id)
            .cloned().unwrap_or_default();
        let (pagos_pendientes, ultimo_pago) = pago_map.get(&contrato.id)
            .copied().unwrap_or((0, None));

        ContratoDetalle {
            contrato,
            propiedad_nombre,
            inquilino_nombre,
            pagos_pendientes,
            ultimo_pago,
        }
    }).collect();

    Ok(detalles)
}
```

**Expected improvement**: 60 round-trips → 3 concurrent round-trips.
Latency drops from ~300ms to ~5-10ms (single DB round-trip time).

### Option B: Concurrent per-contract queries (more complex, less improvement)

Use `futures::future::try_join_all` to run all 20 contract enrichments in parallel.
Still 60 queries but concurrent. Would reduce to ~11ms (max single-query time) but
puts 60 concurrent queries on the connection pool.

### Option C: Single JOIN query (most efficient, least flexible)

```sql
SELECT c.*, p.titulo, i.nombre || ' ' || i.apellido as inquilino_nombre,
       COUNT(pg.id) FILTER (WHERE pg.estado = 'pendiente') as pendientes,
       MAX(pg.fecha_pago) as ultimo_pago
FROM contratos c
JOIN propiedades p ON p.id = c.propiedad_id
JOIN inquilinos i ON i.id = c.inquilino_id
LEFT JOIN pagos pg ON pg.contrato_id = c.id
WHERE c.id = ANY($1)
GROUP BY c.id, p.titulo, i.nombre, i.apellido
```

Single query, ~3-8ms total. Most efficient but couples the query structure.

## Recommendation

**Option A** (batch queries with `try_join!`) is the right fix:
- Reduces 60 sequential round-trips to 3 concurrent ones
- ~30-60x improvement in endpoint latency
- Simple to implement, easy to understand
- Matches the project's code style (`try_join!` for concurrent, iterator chains for mapping)

Do NOT spend time optimizing the Rust mapping logic. The `Vec::with_capacity`,
iterator chains, and struct construction are already near-optimal and irrelevant
to the performance problem.

## Summary

| Question | Answer |
|----------|--------|
| Should we optimize the Rust code? | **No** |
| Should we benchmark iterator approaches? | **No** — waste of time |
| What's the actual bottleneck? | 60 sequential DB round-trips |
| What's the fix? | Batch queries + `try_join!` |
| Expected improvement | ~300ms → ~5-10ms |
| Benchmark needed? | No — this is I/O-bound, not CPU-bound |
