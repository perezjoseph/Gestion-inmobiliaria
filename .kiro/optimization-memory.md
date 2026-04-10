# Optimization Memory

> Auto-maintained by the optimization agent. Last updated: 2026-04-09

## Previously Identified Issues

| File | Category | Description | Fingerprint | First Seen | Occurrences | Status |
|------|----------|-------------|-------------|------------|-------------|--------|
| backend/src/services/dashboard.rs | performance | fetches all active contratos into memory to sum monto_mensual instead of using database aggregation | backend/src/services/dashboard.rs::performance::fetches all active contratos into memory to sum monto_mensual instead of using database aggregation | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/dashboard.rs | performance | calls mark_overdue on every dashboard stats request causing write operation on reads | backend/src/services/dashboard.rs::performance::calls mark_overdue on every dashboard stats request causing write operation on reads | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/contratos.rs | performance | duplicate propiedad lookup in create — fetches propiedad twice within same transaction | backend/src/services/contratos.rs::performance::duplicate propiedad lookup in create — fetches propiedad twice within same transaction | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/contratos.rs | maintainability | existing.clone() in update creates unnecessary full model clone | backend/src/services/contratos.rs::maintainability::existing.clone() in update creates unnecessary full model clone | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/inquilinos.rs | performance | list endpoint returns all records without pagination | backend/src/services/inquilinos.rs::performance::list endpoint returns all records without pagination | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/contratos.rs | performance | list endpoint returns all records without pagination | backend/src/services/contratos.rs::performance::list endpoint returns all records without pagination | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/pagos.rs | performance | list endpoint returns all records without pagination | backend/src/services/pagos.rs::performance::list endpoint returns all records without pagination | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/contratos.rs | maintainability | duplicates can_write, can_delete, format_date_display already defined in utils.rs | frontend/src/pages/contratos.rs::maintainability::duplicates can_write, can_delete, format_date_display already defined in utils.rs | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/pagos.rs | maintainability | duplicates can_write, can_delete, format_date_display already defined in utils.rs | frontend/src/pages/pagos.rs::maintainability::duplicates can_write, can_delete, format_date_display already defined in utils.rs | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/inquilinos.rs | maintainability | duplicates can_write, can_delete already defined in utils.rs | frontend/src/pages/inquilinos.rs::maintainability::duplicates can_write, can_delete already defined in utils.rs | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/app.rs | performance | cors::permissive allows all origins in production — security and performance concern | backend/src/app.rs::performance::cors::permissive allows all origins in production — security and performance concern | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/contratos.rs | performance | fetches all propiedades with perPage=1000 to populate dropdown | frontend/src/pages/contratos.rs::performance::fetches all propiedades with perpage=1000 to populate dropdown | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/dashboard.rs | performance | fires four sequential api calls instead of parallel with join_all | frontend/src/pages/dashboard.rs::performance::fires four sequential api calls instead of parallel with join_all | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/services/api.rs | maintainability | four api functions share nearly identical request/response logic | frontend/src/services/api.rs::maintainability::four api functions share nearly identical request/response logic | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/services/contratos.rs | maintainability | update function does not validate overlap when fecha_fin changes on active contract | backend/src/services/contratos.rs::maintainability::update function does not validate overlap when fecha_fin changes on active contract | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/contratos.rs | performance | fetches all contratos with perPage=1000 instead of paginating | frontend/src/pages/contratos.rs::performance::fetches all contratos with perpage=1000 instead of paginating | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/pagos.rs | performance | fetches all pagos without pagination and contratos dropdown with perPage=1000 | frontend/src/pages/pagos.rs::performance::fetches all pagos without pagination and contratos dropdown with perpage=1000 | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/pages/dashboard.rs | performance | fetches all contratos with perPage=1000 just to show 5 expiring | frontend/src/pages/dashboard.rs::performance::fetches all contratos with perpage=1000 just to show 5 expiring | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| frontend/src/components/common/toast.rs | performance | creates a new timeout closure per toast on every render without cleanup | frontend/src/components/common/toast.rs::performance::creates a new timeout closure per toast on every render without cleanup | 2026-04-09 | 1 | RESOLVED 2026-04-09 |
| backend/src/config.rs | maintainability | config tests fail due to dotenvy loading .env file and mutex poisoning on panic | backend/src/config.rs::maintainability::config tests fail due to dotenvy loading .env file and mutex poisoning on panic | 2026-04-09 | 1 | RESOLVED 2026-04-09 |

## Recurring Patterns

| Pattern | Category | Occurrences | Files Affected |
|---------|----------|-------------|----------------|
| Missing pagination on list endpoints | performance | 3 | inquilinos, contratos, pagos services (RESOLVED — all use SeaORM paginate()) |
| Frontend pages fetching with perPage=1000 | performance | 5 | contratos.rs, pagos.rs, dashboard.rs pages (RESOLVED — capped at 20/100, proper pagination added) |

## Project-Specific Insights

- Backend uses rust_decimal::Decimal for monetary fields; frontend uses f64 with custom deserializers — keep these in sync.
- All four backend list endpoints (propiedades, inquilinos, contratos, pagos) implement pagination via SeaORM paginate().
- The dashboard service performs a write (mark_overdue) on every read — consider moving to a scheduled task. (RESOLVED: mark_overdue removed from dashboard)
- SeaORM entity models use DateTimeWithTimeZone; conversions to DateTime<Utc> via .into() are used throughout services.
- Frontend pages contratos.rs and pagos.rs redefine helpers already in utils.rs — consolidation needed. (RESOLVED: all pages now import from utils.rs)
- CORS is now configurable via CORS_ORIGIN env var; falls back to permissive only when unset (dev mode).
- Config tests that manipulate env vars are unreliable when dotenvy loads .env — avoid testing "missing var" scenarios that dotenvy overrides.
- Frontend API module uses a shared send_request helper to eliminate duplicated request/response/auth logic across api_get, api_post, api_put, api_delete.
- Toast component uses a dedicated ToastItem child component with use_effect_with for proper timeout lifecycle management.
