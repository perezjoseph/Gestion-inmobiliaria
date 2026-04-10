# Optimization Memory

> Auto-maintained by the optimization agent. Last updated: 2026-04-09

## Previously Identified Issues

| File | Category | Description | Fingerprint | First Seen | Occurrences | Status |
|------|----------|-------------|-------------|------------|-------------|--------|
| backend/src/services/dashboard.rs | performance | fetches all active contratos into memory to sum monto_mensual instead of using database aggregation | backend/src/services/dashboard.rs::performance::fetches all active contratos into memory to sum monto_mensual instead of using database aggregation | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/dashboard.rs | performance | calls mark_overdue on every dashboard stats request causing write operation on reads | backend/src/services/dashboard.rs::performance::calls mark_overdue on every dashboard stats request causing write operation on reads | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/contratos.rs | performance | duplicate propiedad lookup in create — fetches propiedad twice within same transaction | backend/src/services/contratos.rs::performance::duplicate propiedad lookup in create — fetches propiedad twice within same transaction | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/contratos.rs | maintainability | existing.clone() in update creates unnecessary full model clone | backend/src/services/contratos.rs::maintainability::existing.clone() in update creates unnecessary full model clone | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/inquilinos.rs | performance | list endpoint returns all records without pagination | backend/src/services/inquilinos.rs::performance::list endpoint returns all records without pagination | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/contratos.rs | performance | list endpoint returns all records without pagination | backend/src/services/contratos.rs::performance::list endpoint returns all records without pagination | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/pagos.rs | performance | list endpoint returns all records without pagination | backend/src/services/pagos.rs::performance::list endpoint returns all records without pagination | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/pages/contratos.rs | maintainability | duplicates can_write, can_delete, format_date_display already defined in utils.rs | frontend/src/pages/contratos.rs::maintainability::duplicates can_write, can_delete, format_date_display already defined in utils.rs | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/pages/pagos.rs | maintainability | duplicates can_write, can_delete, format_date_display already defined in utils.rs | frontend/src/pages/pagos.rs::maintainability::duplicates can_write, can_delete, format_date_display already defined in utils.rs | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/pages/inquilinos.rs | maintainability | duplicates can_write, can_delete already defined in utils.rs | frontend/src/pages/inquilinos.rs::maintainability::duplicates can_write, can_delete already defined in utils.rs | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/app.rs | performance | cors::permissive allows all origins in production — security and performance concern | backend/src/app.rs::performance::cors::permissive allows all origins in production — security and performance concern | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/pages/contratos.rs | performance | fetches all propiedades with perPage=1000 to populate dropdown | frontend/src/pages/contratos.rs::performance::fetches all propiedades with perpage=1000 to populate dropdown | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/pages/dashboard.rs | performance | fires four sequential api calls instead of parallel with join_all | frontend/src/pages/dashboard.rs::performance::fires four sequential api calls instead of parallel with join_all | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/services/api.rs | maintainability | four api functions share nearly identical request/response logic | frontend/src/services/api.rs::maintainability::four api functions share nearly identical request/response logic | 2026-04-09 | 1 | UNRESOLVED |
| backend/src/services/contratos.rs | maintainability | update function does not validate overlap when fecha_fin changes on active contract | backend/src/services/contratos.rs::maintainability::update function does not validate overlap when fecha_fin changes on active contract | 2026-04-09 | 1 | UNRESOLVED |
| frontend/src/components/common/toast.rs | performance | creates a new timeout closure per toast on every render without cleanup | frontend/src/components/common/toast.rs::performance::creates a new timeout closure per toast on every render without cleanup | 2026-04-09 | 1 | UNRESOLVED |

## Recurring Patterns

| Pattern | Category | Occurrences | Files Affected |
|---------|----------|-------------|----------------|
| Missing pagination on list endpoints | performance | 3 | inquilinos, contratos, pagos services |
| Duplicated utility functions across pages | maintainability | 3 | contratos.rs, pagos.rs, inquilinos.rs pages |

## Project-Specific Insights

- Backend uses rust_decimal::Decimal for monetary fields; frontend uses f64 with custom deserializers — keep these in sync.
- Only propiedades service implements pagination; other list endpoints return unbounded results.
- The dashboard service performs a write (mark_overdue) on every read — consider moving to a scheduled task.
- SeaORM entity models use DateTimeWithTimeZone; conversions to DateTime<Utc> via .into() are used throughout services.
- Frontend pages contratos.rs and pagos.rs redefine helpers already in utils.rs — consolidation needed.
