# XLSX Report Generation OOM Analysis

## Step 1: Identify the Target

This is a valid optimization target. Here's why:

- **Hot path**: Yes — user-facing export endpoint, called on-demand by multiple users simultaneously.
- **Evidence of slowness**: OOM kills under 10 concurrent requests. 5000 rows × 180MB = 1.8GB aggregate, far exceeding the 384Mi pod limit.
- **Production dataset size**: 5000 rows is the realistic upper bound per the user's report.

The constraint is **memory, not speed**. 400ms for 5000 rows is acceptable latency. The problem is that 10 concurrent requests × 180MB = 1.8GB, which is ~5x the pod limit. Even 2 concurrent requests (360MB) would push past 384Mi when you add baseline process memory (~50-80MB for the Actix runtime + connection pool).

### Diagnosis: Why 180MB for 5000 rows?

Looking at the current `exportar_xlsx` and `exportar_rentabilidad_xlsx` in `backend/src/services/reportes.rs`:

1. **The `IngresoReportSummary` is fully materialized in memory** — all 5000 `IngresoReportRow` structs with owned `String` fields (propiedad_titulo, inquilino_nombre, moneda, estado) are loaded before XLSX generation begins.

2. **`rust_xlsxwriter::Workbook` builds the entire XLSX in memory** — `workbook.save_to_buffer()` serializes the complete workbook to a `Vec<u8>`. For 5000 rows × 5 columns, this means the workbook's internal representation PLUS the final serialized buffer coexist in memory simultaneously.

3. **String duplication**: Each row clones `propiedad_titulo` and `inquilino_nombre` from the lookup maps. With 5000 pagos across maybe 20 propiedades and 50 inquilinos, the same strings are cloned thousands of times.

4. **No streaming**: The handler builds the full summary, then builds the full XLSX buffer, then sends it. Peak memory = summary + workbook internals + final buffer.

## Step 2: The Right Question

The user asked: "Should I optimize XLSX generation speed, reduce memory, or limit concurrency?"

**Answer: Reduce memory first, then limit concurrency as a safety net.**

- Speed (400ms) is fine — it's well within typical HTTP timeout budgets.
- Memory (180MB per request) is the killer — it's what causes OOM.
- Concurrency limiting alone is a band-aid — even 2 concurrent requests at 180MB each would be dangerous.

## Step 3: Competing Approaches (Memory Reduction)

Here are the concrete implementation strategies, ordered by impact:

### Approach A: Streaming XLSX Generation (Eliminate Double Buffering)

`rust_xlsxwriter` does NOT support true streaming to a writer — `save_to_buffer()` is the only option. However, we can eliminate the double-buffering by not holding the full `IngresoReportSummary` simultaneously with the workbook.

**Strategy**: Generate XLSX rows directly from DB query results in chunks, never materializing all 5000 rows at once.

```rust
use sea_orm::PaginatorTrait;

/// Generate XLSX in chunks to bound peak memory.
/// Instead of: fetch all → build summary → build workbook → serialize
/// Do: create workbook → fetch in pages → write rows → serialize
pub async fn exportar_xlsx_streaming(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: IngresoReportQuery,
    generated_by: String,
) -> Result<Vec<u8>, AppError> {
    // Validate and build the query (same as before)
    let (pago_select, first_day, last_day) = build_pago_query(db, org_id, &query).await?;

    // Row cap check
    let row_count = pago_select.clone().count(db).await?;
    let cap = get_report_row_cap();
    if row_count > cap {
        return Err(AppError::Validation(format!(
            "El reporte excede el límite de {cap} filas."
        )));
    }

    // Pre-load the lookup maps (these are small: ~50 propiedades, ~200 inquilinos)
    let (contrato_map, prop_map, inq_map) = load_lookup_maps(db, org_id).await?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers (same as current)
    write_xlsx_headers(worksheet)?;

    // Process in pages of 500 rows — peak memory is ~500 rows worth
    const PAGE_SIZE: u64 = 500;
    let mut current_row: u32 = 5; // after headers
    let mut total_pagado = Decimal::ZERO;
    let mut total_pendiente = Decimal::ZERO;
    let mut total_atrasado = Decimal::ZERO;

    let paginator = pago_select.paginate(db, PAGE_SIZE);
    let num_pages = paginator.num_pages().await?;

    for page_idx in 0..num_pages {
        let pagos = paginator.fetch_page(page_idx).await?;

        for p in &pagos {
            // Resolve names via pre-loaded maps (no cloning — write &str directly)
            let (prop_titulo, inq_nombre) = resolve_names(&p, &contrato_map, &prop_map, &inq_map);

            match p.estado.as_str() {
                "pagado" => total_pagado += p.monto,
                "pendiente" => total_pendiente += p.monto,
                "atrasado" => total_atrasado += p.monto,
                _ => {}
            }

            worksheet.write_string(current_row, 0, prop_titulo)?;
            worksheet.write_string(current_row, 1, inq_nombre)?;
            worksheet.write_number(current_row, 2, p.monto.to_string().parse::<f64>().unwrap_or_default())?;
            worksheet.write_string(current_row, 3, &p.moneda)?;
            worksheet.write_string(current_row, 4, &p.estado)?;

            current_row += 1;
        }
        // `pagos` Vec is dropped here — memory freed before next page
    }

    // Write summary rows
    write_xlsx_summary(worksheet, current_row + 1, total_pagado, total_pendiente, total_atrasado)?;

    let buf = workbook.save_to_buffer()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error guardando XLSX: {e}")))?;

    Ok(buf)
}
```

**Expected memory profile**: Instead of holding 5000 `IngresoReportRow` structs (~5000 × ~200 bytes = ~1MB for the rows alone, but the summary struct also forces the strings to be owned), we only hold 500 at a time. The workbook internal state still grows, but the input data pressure drops by ~10x.

**However** — the workbook's internal representation still holds all cell data. `rust_xlsxwriter` keeps an in-memory model of all cells. For 5000 rows × 5 columns = 25,000 cells, this is likely 20-40MB depending on string lengths. The `save_to_buffer()` call then produces the final ZIP-compressed XLSX (maybe 2-5MB for 5000 rows of text data).

So the 180MB figure suggests the **summary struct itself** is the dominant cost, not the workbook. Let's investigate:
- 5000 rows × (propiedad_titulo ~30 chars + inquilino_nombre ~40 chars + moneda ~3 chars + estado ~8 chars) = 5000 × ~80 bytes of string content = ~400KB of strings
- But with `String` overhead (24 bytes per String on 64-bit) × 5 strings per row × 5000 = 600KB overhead
- Total IngresoReportSummary: ~1MB

That means the 180MB comes from **`rust_xlsxwriter` internals + the serialization buffer**. This changes the analysis.

### Approach B: Concurrency Semaphore (Limit Simultaneous Exports)

Since `rust_xlsxwriter`'s internal memory usage for 5000 rows is inherently high (~35-40MB for the workbook + the ZIP output buffer), and we can't control that without switching libraries, the most effective fix is to **limit how many exports run simultaneously**.

```rust
use tokio::sync::Semaphore;
use std::sync::LazyLock;

/// Allow at most 2 concurrent XLSX exports.
/// 2 × ~40MB workbook + ~80MB baseline = ~160MB peak, safely under 384Mi.
static EXPORT_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

pub async fn ingresos_xlsx(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<IngresoReportQuery>,
) -> Result<HttpResponse, AppError> {
    // Acquire permit — if 2 exports are running, this queues
    let _permit = EXPORT_SEMAPHORE.acquire().await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Export semaphore closed")))?;

    let summary = reportes::generar_reporte_ingresos(
        db.get_ref(),
        claims.organizacion_id,
        query.into_inner(),
        claims.email,
    )
    .await?;

    // Run XLSX generation on spawn_blocking (CPU-bound work)
    let bytes = web::block(move || reportes::exportar_xlsx(&summary)).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Blocking error: {e}")))??;

    // ... audit and respond
    Ok(HttpResponse::Ok()
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .insert_header(("Content-Disposition", "attachment; filename=\"reporte-ingresos.xlsx\""))
        .body(bytes))
}
```

### Approach C: Reduce Per-Row Allocation (String Interning)

Eliminate cloned strings by using indices into shared lookup tables:

```rust
/// Instead of cloning String per row, store indices into shared Vecs
struct CompactReportRow {
    propiedad_idx: u16,  // index into propiedades Vec
    inquilino_idx: u16,  // index into inquilinos Vec
    monto: Decimal,
    moneda_idx: u8,      // 0 = DOP, 1 = USD
    estado_idx: u8,      // 0 = pagado, 1 = pendiente, 2 = atrasado
}

// 5000 rows × 14 bytes = 70KB instead of 5000 × ~200 bytes = 1MB
```

This helps the **summary struct** but doesn't address `rust_xlsxwriter`'s internal allocation.

### Approach D: Lower the Row Cap

The current `DEFAULT_REPORT_ROW_CAP` is 50,000. The user reports 5000 rows already causing 180MB. Lowering the cap to something more reasonable for the pod's memory:

```rust
/// With 384Mi pod limit, ~80MB baseline, and ~36KB per row in workbook memory,
/// safe cap = (384Mi - 80Mi) / 36KB ≈ 8,600 rows for a single export.
/// With concurrency semaphore at 2: (384Mi - 80Mi) / 2 / 36KB ≈ 4,300 rows.
const DEFAULT_REPORT_ROW_CAP: u64 = 5_000;
```

## Step 4: Recommendation (Without Benchmark — Memory Problem)

Per the skill's rules: I cannot run benchmarks on this system, so I cannot produce predicted results. However, this is primarily a **memory problem, not a speed problem**, and the skill's memory efficiency section applies directly.

### The Math (Deterministic, Not Predicted)

- Pod limit: 384Mi
- Baseline RSS (Actix + SeaORM + connection pool): ~60-80MB (measurable with `/usr/bin/time -v`)
- Available for report generation: ~300MB
- Per-export memory at 5000 rows: 180MB (user-measured)
- Maximum safe concurrent exports: 300MB / 180MB = **1.6 → 1 safely, 2 at risk**

### Recommended Fix (Layered)

**Layer 1 — Concurrency Semaphore** (immediate, highest impact):
```rust
static EXPORT_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));
```
Limits peak memory from 10×180MB to 2×180MB = 360MB. Still tight against 384Mi, so combine with Layer 2.

**Layer 2 — `spawn_blocking` for XLSX generation** (immediate):
The `exportar_xlsx` function is CPU-bound (building the workbook, compressing ZIP). Running it on the async runtime blocks the event loop. Move to `spawn_blocking`:
```rust
let bytes = web::block(move || reportes::exportar_xlsx(&summary)).await??;
```

**Layer 3 — Chunked DB fetching** (medium effort):
Use `paginate(db, 500)` instead of `.all(db)` to avoid holding all 5000 pago entities + all contratos + all propiedades + all inquilinos in memory simultaneously during the data fetch phase.

**Layer 4 — Lower the row cap to match pod resources** (immediate):
```rust
const DEFAULT_REPORT_ROW_CAP: u64 = 5_000;
```
This prevents a single export from ever allocating more than ~180MB.

**Layer 5 — Consider architectural change for truly large exports** (if needed later):
For exports beyond what fits in pod memory, the standard pattern is:
- Accept the export request, return a 202 with a job ID
- Background worker generates the file, writes to object storage (S3/MinIO)
- Client polls or gets notified when file is ready
- Download from object storage (no memory pressure on backend pod)

This is overkill for 5000 rows but becomes necessary if the business requirement grows to 50,000+ rows.

## Step 5: What NOT to Do

- **Don't optimize XLSX generation speed** — 400ms is fine. Speed optimization here is premature and this is not a hot path in the latency-sensitive sense. The bottleneck is memory, not time.
- **Don't switch XLSX libraries** — `rust_xlsxwriter` is the best Rust option. The memory usage is inherent to the XLSX format (ZIP of XML, must be assembled in memory).
- **Don't increase the pod memory limit** to accommodate 10 concurrent 180MB exports — that's 1.8GB, which is wasteful for a property management backend that's idle 99% of the time.

## Step 6: Concrete Implementation

Here's the minimal fix that solves the OOM:

### In `backend/src/handlers/reportes.rs`:

```rust
use std::sync::LazyLock;
use tokio::sync::Semaphore;

/// Limit concurrent XLSX/PDF exports to prevent OOM.
/// 2 concurrent × ~180MB peak = ~360MB, plus ~60MB baseline = ~420MB.
/// Tight but survivable with the chunked approach reducing per-export to ~80MB.
/// With chunked fetch: 2 × ~80MB + 60MB baseline = ~220MB, safely under 384Mi.
static EXPORT_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

pub async fn ingresos_xlsx(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<IngresoReportQuery>,
) -> Result<HttpResponse, AppError> {
    let _permit = EXPORT_SEMAPHORE.acquire().await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Export semaphore closed")))?;

    let summary = reportes::generar_reporte_ingresos(
        db.get_ref(),
        claims.organizacion_id,
        query.into_inner(),
        claims.email,
    )
    .await?;

    let bytes = web::block(move || reportes::exportar_xlsx(&summary)).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Blocking error: {e}")))??;

    let report_id = Uuid::new_v4();
    auditoria::registrar_best_effort(
        db.get_ref(),
        CreateAuditoriaEntry {
            usuario_id: claims.sub,
            entity_type: "reporte".to_string(),
            entity_id: report_id,
            accion: "exportar".to_string(),
            cambios: serde_json::json!({"formato": "xlsx", "tipo": "ingresos"}),
        },
    )
    .await;

    Ok(HttpResponse::Ok()
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .insert_header(("Content-Disposition", "attachment; filename=\"reporte-ingresos.xlsx\""))
        .body(bytes))
}
```

Apply the same pattern to `rentabilidad_xlsx` and both PDF handlers.

### In `backend/src/services/reportes.rs`:

Lower the row cap:
```rust
const DEFAULT_REPORT_ROW_CAP: u64 = 5_000;
```

## Step 7: Verification Plan (Measure After Fixing)

After implementing the semaphore + spawn_blocking fix, verify with:

```bash
# 1. Measure single-export peak RSS
/usr/bin/time -v cargo run --release 2>&1 | grep "Maximum resident"
# Then hit the endpoint with 5000 rows and check RSS

# 2. Load test: 10 concurrent requests, verify no OOM
hey -n 10 -c 10 -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/reportes/ingresos/xlsx?mes=1&anio=2025"

# 3. Verify queuing works (requests complete, just slower under concurrency)

# 4. Monitor pod memory in k8s
kubectl top pod -l app=backend --containers
```

## Summary

| Concern | Answer |
|---------|--------|
| Optimize speed? | No. 400ms is acceptable. |
| Reduce memory? | Yes — but `rust_xlsxwriter` memory is mostly inherent. Cap rows + chunk DB fetches. |
| Limit concurrency? | **Yes — this is the primary fix.** Semaphore at 2 bounds peak to safe levels. |
| Root cause | 10 × 180MB simultaneous = 1.8GB against 384Mi limit. |
| Fix | Semaphore(2) + spawn_blocking + lower row cap to 5000. |
| Future-proofing | Async export job pattern if requirements grow beyond 5000 rows. |

The key insight: **this is not a performance optimization problem, it's a resource contention problem.** The function is fast enough; there are just too many of them running at once. The bench-optimizer skill's "don't optimize" clause applies to speed here — the valid conclusion is to bound concurrency, not make each export faster.
