# Measuring PDF Export Memory Usage

Your concern is valid: `exportar_pdf` builds the entire document in memory (font data + table layout + rendered PDF buffer), and under concurrent requests that multiplies. The only way to know if it's safe within your 384Mi limit is to measure it.

I cannot tell you how much memory it uses — that requires running the measurement. Here's how to set it up.

## Step 1: Is This Worth Profiling?

Yes. This checks the boxes:

- **Constrained environment**: 384Mi pod limit, OOM killer is unforgiving
- **Concurrency multiplier**: N simultaneous report requests = N × per-call allocation
- **Known allocation-heavy path**: `include_bytes!` for 4 font files (copied via `.to_vec()` each call), a `Vec<u8>` buffer growing as the PDF renders, plus genpdf's internal document tree for 500 rows

This is a legitimate memory profiling target.

## Step 2: Add `dhat` to Dev Dependencies

```toml
# backend/Cargo.toml — add under [dev-dependencies]
[dev-dependencies]
dhat = "0.3"
proptest.workspace = true
zip = "8"
actix-http = "3"
```

Also ensure release profile has debug symbols for meaningful backtraces:

```toml
# backend/Cargo.toml (or workspace Cargo.toml)
[profile.release]
debug = 1
```

## Step 3: Write a Memory Budget Test

Create `backend/tests/pdf_memory_budget.rs`:

```rust
//! Memory budget test for PDF export.
//! Must be in its own file — dhat uses global state.
//!
//! Run with: cargo test --release -p realestate-backend --test pdf_memory_budget

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use chrono::Utc;
use rust_decimal::Decimal;

// Mirror the production struct since we can't easily import from the main crate
// without pulling in all dependencies. Alternatively, make the types and
// exportar_pdf function accessible from a lib target.
use realestate_backend::models::reporte::{IngresoReportRow, IngresoReportSummary};
use realestate_backend::services::reportes::exportar_pdf;

/// Generate realistic report data matching production characteristics:
/// - 500 rows (the production size you mentioned)
/// - Mix of estados: 70% pagado, 20% pendiente, 10% atrasado
/// - Realistic string lengths for property titles and tenant names
fn generate_test_summary(row_count: usize) -> IngresoReportSummary {
    let mut rows = Vec::with_capacity(row_count);
    let mut total_pagado = Decimal::ZERO;
    let mut total_pendiente = Decimal::ZERO;
    let mut total_atrasado = Decimal::ZERO;

    for i in 0..row_count {
        let estado = match i % 10 {
            0 => "atrasado",
            1..=2 => "pendiente",
            _ => "pagado",
        };
        let monto = Decimal::new(15_000 + (i as i64 % 50) * 1_000, 0);

        match estado {
            "pagado" => total_pagado += monto,
            "pendiente" => total_pendiente += monto,
            "atrasado" => total_atrasado += monto,
            _ => {}
        }

        rows.push(IngresoReportRow {
            propiedad_titulo: format!("Apartamento en Torre {} Piso {}", i / 20 + 1, i % 20 + 1),
            inquilino_nombre: format!("Inquilino {} Apellido {}", i, i * 2),
            monto,
            moneda: if i % 5 == 0 { "USD".into() } else { "DOP".into() },
            estado: estado.into(),
        });
    }

    IngresoReportSummary {
        rows,
        total_pagado,
        total_pendiente,
        total_atrasado,
        tasa_ocupacion: 75.0,
        generated_at: Utc::now(),
        generated_by: "admin@example.com".into(),
    }
}

#[test]
fn pdf_export_500_rows_memory_profile() {
    let _profiler = dhat::Profiler::builder().testing().build();

    let summary = generate_test_summary(500);
    let _pdf_bytes = exportar_pdf(&summary).expect("PDF export should succeed");

    let stats = dhat::HeapStats::get();

    // Print actual measurements — this is what we need to see
    eprintln!("=== PDF Export Memory Profile (500 rows) ===");
    eprintln!("Peak heap (max live bytes): {:.1} KB", stats.max_bytes as f64 / 1024.0);
    eprintln!("Total allocated: {:.1} KB", stats.total_bytes as f64 / 1024.0);
    eprintln!("Total allocation count: {}", stats.total_blocks);
    eprintln!("Peak live blocks: {}", stats.max_blocks);
    eprintln!("============================================");

    // FIRST RUN: Comment out the assertions below, run the test, read the
    // actual numbers from stderr, then set budgets at ~1.2x the measured values.
    //
    // UNCOMMENT AND FILL IN AFTER FIRST RUN:
    // assert!(
    //     stats.max_bytes < MEASURED_PEAK * 1.2,
    //     "Peak heap {:.1}KB exceeds budget. Regression detected.",
    //     stats.max_bytes as f64 / 1024.0
    // );
}

/// Simulates concurrent pressure: measures memory for multiple PDFs in sequence.
/// In production, these would be concurrent — so total memory pressure is
/// approximately this value × concurrent_requests.
#[test]
fn pdf_export_concurrent_simulation() {
    let _profiler = dhat::Profiler::builder().testing().build();

    // Simulate 5 concurrent requests (sequential here, but measures cumulative allocation)
    let summary = generate_test_summary(500);
    for _ in 0..5 {
        let _pdf_bytes = exportar_pdf(&summary).expect("PDF export should succeed");
        // In production each request would hold its own buffer simultaneously.
        // This test shows per-call allocation; multiply by expected concurrency.
    }

    let stats = dhat::HeapStats::get();

    eprintln!("=== Concurrent Simulation (5 sequential calls, 500 rows each) ===");
    eprintln!("Peak heap (max live bytes): {:.1} KB", stats.max_bytes as f64 / 1024.0);
    eprintln!("Total allocated across 5 calls: {:.1} KB", stats.total_bytes as f64 / 1024.0);
    eprintln!("Avg per call: {:.1} KB", stats.total_bytes as f64 / 5.0 / 1024.0);
    eprintln!("=================================================================");

    // After measuring, verify:
    // per_call_peak × max_concurrent_requests < 384Mi pod limit (minus baseline RSS)
    //
    // Example decision framework:
    //   If per-call peak = 2MB and you expect 10 concurrent requests:
    //   2MB × 10 = 20MB just for PDF generation (probably fine)
    //
    //   If per-call peak = 30MB and you expect 10 concurrent:
    //   30MB × 10 = 300MB → dangerously close to 384Mi → needs optimization
}
```

## Step 4: Run It

```bash
cargo test --release -p realestate-backend --test pdf_memory_budget -- --nocapture
```

The `--nocapture` flag is critical — otherwise you won't see the `eprintln!` output with the actual numbers.

**Note**: If `exportar_pdf` and the model types aren't publicly accessible from integration tests, you may need to either:
1. Add `pub` visibility to the function and re-export the types from `lib.rs`, or
2. Move this test to a unit test within `services/reportes.rs` (but dhat's global allocator requires being at crate root, so an integration test is cleaner).

## Step 5: Interpret Results and Decide

Once you have the numbers, apply this decision framework:

| Measured per-call peak | × 10 concurrent | vs 384Mi budget | Action |
|------------------------|-----------------|-----------------|--------|
| < 5MB | < 50MB | Safe | Document baseline, set regression budget, done |
| 5–20MB | 50–200MB | Caution | Add concurrency limiting (e.g., `tokio::sync::Semaphore`) |
| > 20MB | > 200MB | Danger | Optimize: stream PDF to disk, reuse font data, batch rows |

Things to factor in:
- **Baseline RSS** of your backend process (before any PDF requests). Subtract this from 384Mi to get your actual headroom.
- **Font data**: `load_font_family()` calls `include_bytes!(...).to_vec()` four times per call — that's 4 heap copies of embedded font data. If each font is ~100KB, that's ~400KB per request just for fonts.
- **The output buffer**: `let mut buf: Vec<u8> = Vec::new()` starts empty and grows via reallocation as the PDF renders. For 500 rows this could be significant.

## What NOT To Do

I am not going to predict what the numbers will be. The skill's entire purpose is to replace guessing with measurement. Font rendering, PDF internal structures, and genpdf's allocation patterns are implementation details that can't be reliably predicted from code inspection alone.

Run the test. Read the numbers. Then decide.
