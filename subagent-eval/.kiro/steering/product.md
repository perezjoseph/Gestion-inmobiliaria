---
inclusion: always
---

# Product Domain

Dominican Republic property management platform for property managers.

## Roles & Permissions

| Role | Write | User Mgmt | Scope |
|---|---|---|---|
| `admin` | yes | yes | full access |
| `gerente` | yes | no | properties, tenants, contracts, payments, expenses, maintenance |
| `visualizador` | no | no | read-only on all data |

Enforce via `AdminOnly` / `WriteAccess` extractors in handlers. Every write endpoint requires `admin` or `gerente`. User management endpoints require `admin` only.

## Entity Relationships

```
Usuario (email unique, rol, activo)
  └─ audits all mutations

Propiedad (titulo, direccion, precio + moneda, estado)
  ├─ has many Unidad (numero_unidad, precio + moneda, estado)
  ├─ has many Contrato
  ├─ has many Gasto
  └─ has many SolicitudMantenimiento

Inquilino (nombre, apellido, cedula unique)
  └─ has many Contrato

Contrato (propiedad_id, inquilino_id, fecha_inicio..fecha_fin, monto_mensual, moneda, estado)
  └─ has many Pago

Pago (contrato_id, monto, moneda, fecha_vencimiento, fecha_pago?, estado)

Gasto (propiedad_id, unidad_id?, categoria, monto, moneda, fecha_gasto, estado)

SolicitudMantenimiento (propiedad_id, unidad_id?, inquilino_id?, estado, prioridad, costo?)
  └─ has many NotaMantenimiento

Documento (entity_type, entity_id, filename, file_path) — polymorphic attachment
```

## Business Invariants

These rules must hold at all times. Violations are bugs.

- **No overlapping active contracts**: a propiedad cannot have two contratos with `estado = "activo"` whose date ranges overlap. Validate before insert and update.
- **Cedula uniqueness**: inquilino.cedula is unique across the system. Reject duplicates at the service layer with a clear error.
- **Email uniqueness**: usuario.email is unique. Reject duplicates at the service layer.
- **Currency consistency**: every monetary entity (contrato, pago, gasto, propiedad, unidad) carries its own `moneda` field (`DOP` or `USD`). Never assume currency from context.
- **Payment lateness**: a pago is late when `fecha_pago > fecha_vencimiento` (paid after due) OR `fecha_pago IS NULL AND fecha_vencimiento < today` (unpaid past due).
- **Contrato integrity**: every contrato references exactly one propiedad and one inquilino. A pago always belongs to a contrato.
- **Gasto scope**: a gasto belongs to a propiedad and optionally to a unidad within that propiedad. If unidad_id is set, it must belong to the referenced propiedad.
- **Propiedad estado cascade**: creating an active contrato sets propiedad.estado to `ocupada`. Cancelling or terminating the last active contrato sets it back to `disponible`. This is enforced in the contratos service within the same transaction.

## Estado Values

| Entity | Valid States |
|---|---|
| Propiedad | `disponible`, `ocupada`, `mantenimiento` |
| Contrato | `activo`, `vencido`, `cancelado` + `finalizado` (renewal), `terminado` (early termination) |
| Pago | `pendiente`, `pagado`, `atrasado` |
| Gasto | `pendiente`, `pagado`, `cancelado` |
| SolicitudMantenimiento | `pendiente`, `en_progreso`, `completado` |
| Usuario | `activo` (bool field, not string) |

Prioridad values for mantenimiento: `baja`, `media`, `alta`, `urgente`.
Métodos de pago: `efectivo`, `transferencia`, `cheque`, `tarjeta`.

## Localization

- All user-facing text in Spanish.
- Dates: DD/MM/YYYY display, ISO 8601 storage.
- Currency: DOP (Dominican Peso) and USD. Always display with currency symbol and two decimals.
