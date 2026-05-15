# Algorithm Optimization Report: Dashboard Matching

## Summary

Optimized three functions in the dashboard module that suffered from O(n²) and O(n³) complexity due to nested linear scans. All three were reduced to O(n) overall by applying the **transform and conquer** paradigm — pre-building HashMap/HashSet lookup structures before the main iteration.

## Original Complexity

| Function | Original | Pattern |
|---|---|---|
| `generar_resumen` | O(p × c × g) | Triple nested loop: propiedades → contratos → pagos |
| `propiedades_con_atrasos` | O(p × c × g) | Triple nested loop with early break |
| `ocupacion_por_ciudad` | O(cities × n) | Collect unique cities, then re-scan per city |

Where p = propiedades, c = contratos, g = pagos, n = propiedades count.

## Optimized Complexity

| Function | Optimized | Approach |
|---|---|---|
| `generar_resumen` | O(p + c + g) | Pre-aggregate pagos into `HashMap<contrato_id, (pagado, pendiente)>`, group contratos by `propiedad_id`, then single-pass summation |
| `propiedades_con_atrasos` | O(p + c + g) | Build `HashSet<contrato_id>` of contracts with atrasado pagos, group active contratos by `propiedad_id`, then O(1) membership checks |
| `ocupacion_por_ciudad` | O(n) | Single-pass grouping into `HashMap<ciudad, (total, ocupadas)>`, then compute ratios |

## Techniques Applied

1. **HashMap pre-indexing (Transform and Conquer)**: Instead of scanning all contratos for each propiedad, build a `HashMap<propiedad_id, Vec<&Contrato>>` once in O(c), then look up in O(1) per propiedad.

2. **Pre-aggregation**: Instead of scanning all pagos for each contrato, aggregate pago totals by contrato_id in a single O(g) pass. This eliminates the innermost loop entirely.

3. **HashSet membership testing**: For the "has atrasado" check, collect all contrato_ids with atrasado pagos into a HashSet. Membership testing is O(1) amortized vs O(g) linear scan.

4. **Single-pass grouping**: For `ocupacion_por_ciudad`, replace the sort+dedup+rescan pattern with a single HashMap accumulation pass.

## Space Complexity

All optimizations trade additional memory for time:

| Function | Additional Space |
|---|---|
| `generar_resumen` | O(c + g) for the two HashMaps |
| `propiedades_con_atrasos` | O(c + g) for HashMap + HashSet |
| `ocupacion_por_ciudad` | O(cities) for the grouping HashMap |

This is a standard and worthwhile space-time tradeoff given that the data fits in memory (it was already loaded into Vecs).

## Style Compliance

- Uses iterator chains over explicit loops where idiomatic
- Uses `Vec::with_capacity` for pre-sized allocations
- Borrows (`&[T]`, `&str`) instead of cloning where possible
- No `unwrap()`/`expect()` in logic paths
- Follows `snake_case` naming conventions
