# Fixing OOM on XLSX Report Generation Under Concurrency

## The Core Problem

Your numbers tell the story clearly:

| Rows | Time | Memory |
|------|------|--------|
| 100 | 8ms | 2MB |
| 5000 | 400ms | 180MB |

That's ~36KB/row at 5000 rows. With 10 concurrent requests: 10 × 180MB = 1.8GB peak, far exceeding your 384Mi pod limit. The OOM is inevitable.

## Answer: You Need Memory Reduction AND Concurrency Limiting

This isn't an either/or. Speed optimization won't help — the pod dies from memory pressure, not slowness. You need both:

1. **Reduce per-request memory** (the biggest win)
2. **Limit concurrency** (the safety net)

## Strategy 1: Streaming XLSX Generation (Primary Fix)

The `rust_xlsxwriter` crate builds the entire workbook in memory before serializing. For large reports, switch to a streaming approach:

```rust
use rust_xlsxwriter::Workbook;
use tokio::sync::Semaphore;
use std::sync::Arc;

// rust_xlsxwriter supports worksheet.set_row() incrementally,
// but the workbook still buffers. The key optimization is to
// write to a temp file instead of an in-memory buffer.

pub async fn generate_report_streaming(
    data: Vec<ReportRow>,
) -> Result<tempfile::NamedTempFile, AppError> {
    // Write directly to a temp file rather than Vec<u8>
    let tmp = tempfile::NamedTempFile::new()?;
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write header
    for (col, header) in HEADERS.iter().enumerate() {
        worksheet.write_string(0, col as u16, header)?;
    }

    // Write rows in chunks to allow periodic memory release
    for (row_idx, row) in data.iter().enumerate() {
        write_row(worksheet, (row_idx + 1) as u32, row)?;
    }

    // Save to the temp file path — avoids holding full XLSX bytes in memory
    workbook.save(tmp.path())?;
    Ok(tmp)
}
```

**Key insight**: `workbook.save()` to a file path uses less peak memory than `workbook.save_to_buffer()` because it doesn't need the final serialized bytes in RAM simultaneously with the internal model.

## Strategy 2: Chunked Data Loading

Don't load all 5000 rows into memory at once from the database:

```rust
use futures::StreamExt;
use sea_orm::*;

pub async fn generate_report_chunked(
    db: &DatabaseConnection,
    filter: ReportFilter,
) -> Result<tempfile::NamedTempFile, AppError> {
    let tmp = tempfile::NamedTempFile::new()?;
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    write_headers(worksheet)?;

    // Stream rows from DB in pages of 500
    let mut offset = 0u64;
    let page_size = 500u64;
    let mut row_idx = 1u32;

    loop {
        let page = build_query(&filter)
            .offset(offset)
            .limit(page_size)
            .all(db)
            .await?;

        if page.is_empty() {
            break;
        }

        for record in &page {
            write_row(worksheet, row_idx, record)?;
            row_idx += 1;
        }

        offset += page_size;
        // Each page is dropped here, freeing its Vec allocation
    }

    workbook.save(tmp.path())?;
    Ok(tmp)
}
```

This reduces peak DB-side memory from holding all 5000 rows to holding 500 at a time.

## Strategy 3: Concurrency Semaphore (Safety Net)

Even with memory optimizations, you need a hard cap on concurrent report generations:

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

// In your app state / AppState struct:
pub struct AppState {
    pub db: DatabaseConnection,
    pub report_semaphore: Arc<Semaphore>,
    // ...
}

impl AppState {
    pub fn new(db: DatabaseConnection) -> Self {
        // Allow max 2 concurrent report generations
        // 2 × ~90MB (optimized) = 180MB, leaves headroom in 384Mi
        let report_semaphore = Arc::new(Semaphore::new(2));
        Self { db, report_semaphore }
    }
}

// In your handler:
pub async fn export_report(
    state: web::Data<AppState>,
    params: web::Query<ReportParams>,
) -> Result<HttpResponse, AppError> {
    // Acquire permit — blocks if 2 reports already in progress
    let _permit = state.report_semaphore
        .acquire()
        .await
        .map_err(|_| AppError::ServiceUnavailable("Report generation busy".into()))?;

    let file = generate_report_chunked(&state.db, params.into_inner().into()).await?;

    // Stream the file to the client
    let stream = tokio::fs::File::open(file.path()).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .streaming(tokio_util::io::ReaderStream::new(stream)))
}
```

## Strategy 4: Row Limit as a Business Rule

Add a hard cap and offer pagination/async export for large reports:

```rust
const MAX_EXPORT_ROWS: u64 = 10_000;

pub async fn export_report(params: ReportParams) -> Result<HttpResponse, AppError> {
    let count = count_matching_rows(&params).await?;
    if count > MAX_EXPORT_ROWS {
        return Err(AppError::BadRequest(format!(
            "El reporte tiene {} filas. El máximo permitido es {}. \
             Use filtros más específicos.",
            count, MAX_EXPORT_ROWS
        )));
    }
    // proceed...
}
```

## Memory Budget Calculation

With 384Mi total, you need to budget:

| Component | Allocation |
|-----------|-----------|
| Rust runtime + app baseline | ~30MB |
| DB connection pool | ~20MB |
| Normal request handling | ~50MB |
| **Available for reports** | **~280MB** |

With the semaphore set to 2, each report can use up to ~140MB. Your optimized approach (file-backed + chunked loading) should bring a 5000-row report down to ~50-80MB peak, giving comfortable headroom.

## Recommended Implementation Order

1. **Add the semaphore** (immediate relief, 15 minutes of work)
2. **Switch to file-backed save** (`workbook.save(path)` instead of `save_to_buffer()`)
3. **Chunk your DB queries** (paginated loading)
4. **Stream the response** (don't read the entire file back into memory)
5. **Add row limits** with clear Spanish-language error messages

## What NOT to Do

- **Don't just increase the pod memory limit** — this masks the real problem and shifts OOM to larger reports.
- **Don't optimize XLSX write speed** — speed isn't the constraint; memory is. A faster OOM is still an OOM.
- **Don't use CSV as a workaround** — if users need XLSX formatting, give them XLSX. Fix the memory profile instead.

## Optional: Background Job for Very Large Reports

If users eventually need 50k+ row exports, move report generation to an async job:

```rust
// POST /api/reportes/exportar → returns job ID
// GET /api/reportes/exportar/{job_id}/estado → polling endpoint
// GET /api/reportes/exportar/{job_id}/descargar → download when ready
```

This decouples the HTTP request lifecycle from the generation time and lets you run the job with dedicated memory limits in a separate k8s Job or a worker pod.
