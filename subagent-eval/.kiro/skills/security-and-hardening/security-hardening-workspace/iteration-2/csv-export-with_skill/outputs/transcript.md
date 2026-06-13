# CSV Export Endpoint — Security Reasoning Transcript

## Task

Implement `GET /api/v1/reportes/pagos-export` that exports all payments for the authenticated user's organization as CSV, with optional `fecha_inicio` and `fecha_fin` query params.

## Security Considerations Applied (from SKILL.md)

### 1. Authentication — `Claims` extractor

The endpoint uses the `Claims` extractor (read-only endpoint, no mutation). This ensures only authenticated users can trigger the export. No unauthenticated access is possible — the handler won't execute without a valid JWT.

### 2. Multi-Tenant Query Scoping

The service query filters by `pago::Column::OrganizacionId.eq(org_id)` where `org_id` comes from `claims.organizacion_id`. This prevents cross-tenant data leakage — a user can only export their own organization's payments.

### 3. Data Export Security — Date Boundaries

Per the skill's "Data Export Security" section:
- **Default date boundary**: If no dates are provided, defaults to last 12 months (not unbounded).
- **Date range cap**: Maximum 2 years per request to prevent abuse.
- **Row count cap**: 50,000 rows maximum to prevent OOM/DoS.

### 4. Input Validation at Handler Boundary

- `fecha_inicio > fecha_fin` → rejected with `AppError::Validation`
- Date range > 2 years → rejected
- Dates are parsed via serde `NaiveDate` deserialization (invalid formats rejected automatically)

### 5. Rate Limiting Recommendation

The skill says exports should be wrapped with rate limiter (expensive operation). The routes.rs file documents how to add `write_governor` to the reportes scope or a nested sub-scope for export-only routes.

### 6. Audit Logging

Every export is logged with `tracing::info!` including:
- `user_email` — who triggered it
- `organizacion_id` — which tenant
- `fecha_inicio` / `fecha_fin` — what range was requested

This satisfies the skill's requirement: "Log every export with structured fields."

### 7. Content-Disposition Header

Set to `attachment; filename="pagos-export.csv"` so browsers download rather than render. This is explicitly called out in the skill's Data Export Security section.

### 8. Error Response Safety

Internal errors (CSV writer failures) use `AppError::Internal(anyhow::anyhow!(...))` which returns a generic "Error interno del servidor" to the client while logging the real error server-side. No internal details leak.

### 9. No SQL Injection Risk

All queries use SeaORM's query builder with `.filter()` — fully parameterized. No `Statement::from_string()` or `format!()` interpolation.

### 10. No Unbounded Buffering

The combination of date range defaults (12 months), date range cap (2 years), and row count cap (50k) ensures the endpoint cannot be abused to buffer unlimited data into memory.

## What Was NOT Done (and why)

- **No `WriteAccess` extractor**: This is a read-only GET endpoint. The skill says read endpoints use `Claims`.
- **No `#[serde(deny_unknown_fields)]`**: The skill warns against blanket use. Query params with unknown fields are harmlessly ignored.
- **No CORS `expose-headers`**: The skill mentions exposing Content-Disposition via CORS, but that's a global CORS config concern, not per-endpoint. Noted for integration.

## Dependencies

The service uses the `csv` crate for CSV generation. This should be added to `Cargo.toml`:
```toml
csv = "1"
```

This is a well-maintained, widely-used crate with no known advisories.
