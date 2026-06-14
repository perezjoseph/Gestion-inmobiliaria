---
inclusion: always
---

# Product Domain

Dominican Republic property management platform for property managers.

Entity model, business invariants, and estado values: see `product-domain.md` (loads when editing backend, frontend, or android code).

## Roles & Permissions

| Role | Write | User Mgmt | Scope |
|---|---|---|---|
| `admin` | yes | yes | full access |
| `gerente` | yes | no | properties, tenants, contracts, payments, expenses, maintenance |
| `visualizador` | no | no | read-only on all data |

Enforce via `AdminOnly` / `WriteAccess` extractors in handlers. Every write endpoint requires `admin` or `gerente`. User management endpoints require `admin` only.

## Localization

- All user-facing text in Spanish.
- Dates: DD/MM/YYYY display, ISO 8601 storage.
- Currency: DOP (Dominican Peso) and USD. Always display with currency symbol and two decimals.
