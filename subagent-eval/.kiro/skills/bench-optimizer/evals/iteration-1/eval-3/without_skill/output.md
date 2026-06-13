# Measuring Memory Usage of Your PDF Export Function

You have a few complementary approaches here, from local profiling to production observability.

## 1. Local measurement with `jemalloc` + `tikv-jemallocator`

The most precise way to measure per-request allocator stats in Rust is to use jemalloc with its stats API. Add to your `Cargo.toml`:

```toml
[dependencies]
tikv-jemallocator = "0.6"
tikv-jemalloc-ctl = "0.6"
```

In `main.rs`:

```rust
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

Then in your PDF handler, snapshot memory before and after:

```rust
use tikv_jemalloc_ctl::{stats, epoch};

async fn export_pdf(/* ... */) -> Result<HttpResponse, Error> {
    // Advance the jemalloc epoch to refresh stats
    epoch::advance().unwrap();
    let before = stats::allocated::read().unwrap();

    let pdf_bytes = generate_pdf_report(&data).await?;

    epoch::advance().unwrap();
    let after = stats::allocated::read().unwrap();

    tracing::info!(
        allocated_bytes = after - before,
        "PDF export memory delta"
    );

    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .body(pdf_bytes))
}
```

This gives you the heap delta for a single request. Run it with 500 rows and note the number. If one request uses ~50 MiB, then 7 concurrent requests would exceed 384 MiB — that's your OOM boundary.

**Caveat**: `stats::allocated` is process-wide, so under concurrency the deltas overlap. For isolated measurement, run a single request in a test or use approach #2.

## 2. Isolated benchmark with `criterion` and peak RSS

Write a benchmark that exercises the PDF path and measures peak memory:

```rust
// benches/pdf_memory.rs
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let current = ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(current, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;
```

Then in a test or benchmark:

```rust
#[test]
fn measure_pdf_peak_memory() {
    PEAK.store(0, Ordering::Relaxed);
    ALLOCATED.store(0, Ordering::Relaxed);

    let data = generate_test_data(500); // 500 rows
    let _pdf = generate_pdf_report_sync(&data);

    let peak_mib = PEAK.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
    println!("Peak heap usage for 500-row PDF: {peak_mib:.2} MiB");

    // Fail if a single export exceeds safe threshold
    // With 384 MiB limit, leave ~100 MiB for runtime overhead
    // If you want to handle N concurrent exports: (384 - 100) / N
    assert!(peak_mib < 50.0, "Single PDF export exceeds 50 MiB budget");
}
```

This gives you a hard number for one invocation without interference from other allocations.

## 3. Concurrency stress test

Once you know the per-request cost, simulate concurrent load:

```rust
#[tokio::test]
async fn concurrent_pdf_memory_pressure() {
    let handles: Vec<_> = (0..10)
        .map(|_| {
            tokio::spawn(async {
                let data = generate_test_data(500);
                generate_pdf_report(&data).await
            })
        })
        .collect();

    for h in handles {
        h.await.unwrap().unwrap();
    }

    epoch::advance().unwrap();
    let total = stats::allocated::read().unwrap();
    let total_mib = total as f64 / (1024.0 * 1024.0);
    println!("After 10 concurrent PDFs: {total_mib:.2} MiB allocated");
}
```

## 4. Production observability on Kubernetes

In your actual deployment, monitor RSS from the outside:

**Container metrics (preferred):** Use Prometheus with `container_memory_working_set_bytes` from kubelet/cAdvisor. Set an alert when working set exceeds 80% of the limit (307 MiB):

```yaml
# PrometheusRule
- alert: PdfPodMemoryHigh
  expr: container_memory_working_set_bytes{container="backend"} > 322961408
  for: 30s
  annotations:
    summary: "Backend pod approaching OOM (>308 MiB of 384 MiB limit)"
```

**From inside the pod**, read `/proc/self/status`:

```rust
fn resident_memory_mib() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0)
        / 1024.0 // kB -> MiB
}
```

Log this before and after the PDF handler in production to see real-world impact.

## 5. Mitigation strategies (once you have numbers)

Depending on what the measurement shows:

| Finding | Action |
|---------|--------|
| Single request uses >50 MiB | Stream PDF generation instead of buffering the full document in memory. `genpdf` builds in-memory, so consider chunking rows or switching to a streaming PDF writer. |
| Concurrent requests stack up | Add a semaphore to limit concurrent PDF exports (e.g., `tokio::sync::Semaphore` with 3-4 permits) |
| Total fits but is tight | Set resource requests closer to limits so k8s schedules appropriately; add HPA based on memory |
| Way over budget | Generate PDFs in a separate worker pod with higher memory limits, or paginate the export |

Example semaphore guard:

```rust
use tokio::sync::Semaphore;
use std::sync::LazyLock;

static PDF_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(3));

async fn export_pdf(/* ... */) -> Result<HttpResponse, Error> {
    let _permit = PDF_SEMAPHORE.acquire().await.unwrap();
    // ... generate PDF ...
}
```

## Summary

Start with approach #2 (tracking allocator in a test) to get an isolated per-request number. Multiply by your expected concurrency. If `per_request_mib * max_concurrent + baseline > 384`, you need either a concurrency limiter or an architecture change. Then deploy with approach #4 to validate in production.
