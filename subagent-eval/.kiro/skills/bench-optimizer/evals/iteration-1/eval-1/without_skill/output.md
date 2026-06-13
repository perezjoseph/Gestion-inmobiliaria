# Grouping 2000 Payments by `property_id`: HashMap vs Sort+Scan

## Short Answer

For 2000 elements, `HashMap<Uuid, Vec<&Payment>>` is almost certainly the fastest practical approach. A sort-then-scan strategy will be slower due to the cost of comparison-based sorting on 128-bit UUIDs.

## Why HashMap Wins at This Scale

**HashMap approach** — O(n) average time:
- One pass over the data
- Each insert is O(1) amortized (hash + push to vec)
- Uuid hashing is fast (128 bits, no heap allocation)

**Sort + linear scan** — O(n log n) time:
- `sort_unstable_by_key` on Uuid requires ~11 comparisons per element for n=2000 (log₂ 2000 ≈ 11)
- Each comparison compares 16 bytes
- Then a linear scan to collect groups — another O(n) pass
- Total: ~22,000 comparisons + 2,000 element moves + a grouping pass

At n=2000, the HashMap approach does ~2,000 hash operations vs ~22,000 comparisons for sorting. The HashMap wins.

## When Sort+Scan Could Win

Sort-then-scan has better **cache locality** during the grouping phase and avoids HashMap's occasional resizing. It could potentially win if:

- n is very large (hundreds of thousands) AND you reuse the sorted order downstream
- You need the groups in a sorted order anyway
- You're in a no-alloc environment where the hash table overhead matters

None of these typically apply to a 2000-element payment grouping.

## Your Current Approach (Optimized)

```rust
use std::collections::HashMap;
use uuid::Uuid;

fn group_by_property<'a>(payments: &'a [Payment]) -> HashMap<Uuid, Vec<&'a Payment>> {
    let mut groups: HashMap<Uuid, Vec<&Payment>> = HashMap::with_capacity(payments.len() / 4);
    for payment in payments {
        groups.entry(payment.propiedad_id).or_default().push(payment);
    }
    groups
}
```

Key optimization: `with_capacity` avoids rehashing. Estimate the number of unique properties (if you know roughly how many properties exist, use that number instead of `len() / 4`).

## The Sort+Scan Alternative (For Reference)

```rust
fn group_by_property_sorted<'a>(payments: &'a mut [Payment]) -> Vec<(Uuid, &'a [Payment])> {
    payments.sort_unstable_by_key(|p| p.propiedad_id);
    payments
        .chunk_by(|a, b| a.propiedad_id == b.propiedad_id)
        .map(|group| (group[0].propiedad_id, group))
        .collect()
}
```

This returns slices into the sorted array (zero-copy grouping), but requires mutable access and pays the O(n log n) sort cost upfront.

## Recommendation

Stick with `HashMap`. At 2000 elements it's the right tool. The only change worth making is adding `with_capacity` if you haven't already. If you ever scale to 100k+ payments and profiling shows this function as a hotspot, revisit then — but even at that scale, HashMap typically still wins for pure grouping.
