---
inclusion: always
---

# Product Overview

Real estate property management application for the Dominican Republic. Manages property listings, tenant records, lease contracts, and rent payment tracking.

## Target Users

Property managers handling multiple properties, tenants, and rental agreements across the Dominican Republic.

## Roles and Permissions

- `admin` — Full access to all features including user management.
- `gerente` (manager) — Manage properties, tenants, contracts, and payments. No user management.
- `visualizador` (viewer) — Read-only access to all listings, reports, and dashboard.

## Domain Model

Five core entities:

- Usuarios — System users with role-based access. Identified by email (unique).
- Propiedades — Rental properties with location (direccion, ciudad, provincia), type (tipo_propiedad), pricing (precio + moneda), and status (estado).
- Inquilinos — Tenants identified by cedula (unique, Dominican national ID).
- Contratos — Lease agreements linking one propiedad to one inquilino with date range, monthly amount (monto_mensual), and optional deposit.
- Pagos — Individual rent payments tied to a contrato with due date, optional payment date, payment method, and status.

## Business Rules

- A property cannot have overlapping active contracts for the same date range.
- A payment is considered late when the payment date exceeds the due date, or when unpaid and the current date exceeds the due date.
- Currency is tracked per record (DOP or USD).

## Localization

- All UI text must be in Spanish. No English strings in user-facing components.
- Date format: DD/MM/YYYY (Dominican Republic convention).
- Currency display: DOP and USD with proper formatting.
- Error messages should be in Spanish.
