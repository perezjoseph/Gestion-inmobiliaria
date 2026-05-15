# Dashboard Aggregation Optimization Analysis

## Summary

The `ingresos_por_propiedad_mes` function was benchmarked with 4 implementations across 3 dataset sizes. The **sort-then-scan** approach wins at production scale (2000 pagos) with a **53% speedup**, while the **presized numeric key** approach wins at larger scales.

## Benchmark Results

| Implementation | 500 pagos | 2000 pagos (production) | 10000 pagos |
|---|---|---|---|
| **original** | 108.33 µs | 465.28 µs | 2.39 ms |
| **numeric_key** | 77.08 µs | 294.73 µs | 1.06 ms |
| **presized** | 59.56 µs | 326.09 µs | 911.91 µs |
| **sort_scan** | 51.69 µs | 217.68 µs | 986.82 µs |

## Speedup vs Original (production dataset: 2000 pagos)

| Implementation | Time | Speedup |
|---|---|---|
| original | 465.28 µs | baseline |
| numeric_key | 294.73 µs | **1.58x faster** (37% reduction) |
| presized | 326.09 µs | **1.43x faster** (30% reduction) |
| sort_scan | 217.68 µs | **2.14x faster** (53% reduction) |

## Implementations Tested

### 1. Original (baseline)
The existing implementation: single-pass HashMap with `format!("%Y-%m")` String keys per pago.

**Bottleneck**: `chrono::format` allocates a String for every single pago, even when many pagos share the same month. The HashMap<String, f64> inner map also has overhead from String hashing and comparison.

### 2. Numeric Key (`ingresos_numeric_key`)
Replaces the `String` month key with a `(i32, u32)` tuple (year, month). Only formats to String at the very end when building the result vector.

**Why it's faster**: Eliminates ~1400 String allocations (for the ~70% of pagos that are "pagado") during accumulation. Tuple comparison is also cheaper than String comparison.

### 3. Presized (`ingresos_presized`)
Same as numeric key but pre-allocates HashMap capacity based on known production characteristics (50 propiedades, 24 months).

**Why it helps at scale**: Avoids HashMap resizing/rehashing. At 500 pagos it's the fastest HashMap approach because the capacity hints are proportionally more impactful. At 2000 pagos the benefit is less pronounced due to measurement variance.

### 4. Sort-then-Scan (`ingresos_sort_scan`)
Filters, sorts by (propiedad_id, fecha_pago), then does a single linear scan to accumulate. No inner HashMap at all.

**Why it wins at production scale**: After sorting, identical propiedades and months are adjacent, so accumulation is a simple running sum with no hash lookups. The sort cost (O(n log n)) is offset by the elimination of all HashMap overhead for the inner grouping. Cache locality is also better since we're scanning linearly.

## Recommendation

**For the production dataset (~2000 pagos, ~50 propiedades):** Use `sort_scan`.

- 2.14x faster than the original (217 µs vs 465 µs)
- Simpler memory profile (no nested HashMaps)
- Results are already sorted (no separate sort step at the end)

At 10000 pagos, `presized` edges out `sort_scan` slightly (912 µs vs 987 µs), but the production dataset is ~2000 pagos where `sort_scan` clearly wins.

## Recommended Implementation

```rust
pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    use chrono::Datelike;

    let mut filtered: Vec<&Pago> = pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
        .collect();

    if filtered.is_empty() {
        return HashMap::new();
    }

    filtered.sort_unstable_by(|a, b| {
        a.propiedad_id
            .cmp(&b.propiedad_id)
            .then_with(|| a.fecha_pago.cmp(&b.fecha_pago))
    });

    let mut result: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(50);

    let mut current_prop = filtered[0].propiedad_id;
    let mut current_month = {
        let f = filtered[0].fecha_pago.unwrap();
        (f.year(), f.month())
    };
    let mut current_sum = 0.0_f64;
    let mut months_vec: Vec<(String, f64)> = Vec::with_capacity(24);

    for pago in &filtered {
        let fecha = pago.fecha_pago.unwrap();
        let month = (fecha.year(), fecha.month());

        if pago.propiedad_id != current_prop {
            months_vec.push((format!("{:04}-{:02}", current_month.0, current_month.1), current_sum));
            result.insert(current_prop, months_vec);
            months_vec = Vec::with_capacity(24);
            current_prop = pago.propiedad_id;
            current_month = month;
            current_sum = pago.monto;
        } else if month != current_month {
            months_vec.push((format!("{:04}-{:02}", current_month.0, current_month.1), current_sum));
            current_month = month;
            current_sum = pago.monto;
        } else {
            current_sum += pago.monto;
        }
    }

    months_vec.push((format!("{:04}-{:02}", current_month.0, current_month.1), current_sum));
    result.insert(current_prop, months_vec);

    result
}
```

## Why the Original Was Slow

1. **String allocation per pago**: `format!("%Y-%m")` creates a heap-allocated String for every filtered pago (~1400 allocations). The optimized versions defer formatting to the final output step.
2. **Nested HashMap overhead**: The inner `HashMap<String, f64>` has per-entry overhead (hashing, bucket management, String comparison) that's disproportionate for only ~24 entries per propiedad.
3. **No capacity hints**: The original doesn't pre-size anything, causing multiple reallocations as maps grow.

## Practical Impact

At 465 µs per dashboard load, the original is already fast enough for a single user. But on a shared server handling multiple concurrent dashboard requests, the 2.14x improvement (to 218 µs) reduces CPU time meaningfully. The sort_scan approach also has a more predictable memory profile since it avoids nested HashMap allocations.
