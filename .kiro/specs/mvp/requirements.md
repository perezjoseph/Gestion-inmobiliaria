# Requirements: MVP — Real Estate Property Management

## Overview
Build a functional MVP for a property management application targeting the Dominican Republic market. The system must support property listings, tenant management, lease tracking, and rent payment monitoring with role-based access control. All UI text in Spanish.

## Functional Requirements

### FR-1: Authentication & Authorization
- FR-1.1: Users can register with email, password, name, and role (admin, gerente, visualizador).
- FR-1.2: Users can log in with email and password, receiving a JWT token.
- FR-1.3: Passwords are hashed with Argon2 before storage.
- FR-1.4: JWT middleware protects all routes except login and register.
- FR-1.5: RBAC middleware enforces role-based permissions:
  - admin: full CRUD on all resources + user management.
  - gerente: full CRUD on properties, tenants, contracts, payments.
  - visualizador: read-only access to all resources.
- FR-1.6: Frontend stores JWT in localStorage and attaches it to all API requests.
- FR-1.7: Frontend redirects to login on 401 responses or missing token.

### FR-2: Property Management (Propiedades)
- FR-2.1: CRUD operations for properties via REST API.
- FR-2.2: Property fields: titulo, descripcion, direccion, ciudad, provincia, tipo_propiedad (casa, apartamento, comercial, terreno), habitaciones, banos, area_m2, precio, moneda (DOP/USD), estado (disponible, ocupada, mantenimiento), imagenes (JSONB array of URLs).
- FR-2.3: List properties with pagination (page, per_page query params).
- FR-2.4: Filter properties by ciudad, provincia, tipo_propiedad, estado, and price range.
- FR-2.5: Frontend page with table listing, create/edit form, and delete confirmation.

### FR-3: Tenant Management (Inquilinos)
- FR-3.1: CRUD operations for tenants via REST API.
- FR-3.2: Tenant fields: nombre, apellido, email, telefono, cedula (national ID), contacto_emergencia, notas.
- FR-3.3: Search tenants by nombre, apellido, or cedula.
- FR-3.4: Frontend page with table listing, create/edit form, and delete confirmation.

### FR-4: Lease/Contract Management (Contratos)
- FR-4.1: CRUD operations for contracts via REST API.
- FR-4.2: Contract fields: propiedad_id (FK), inquilino_id (FK), fecha_inicio, fecha_fin, monto_mensual, deposito, moneda, estado (activo, vencido, terminado).
- FR-4.3: Creating a contract sets the linked property status to "ocupada".
- FR-4.4: Terminating/expiring a contract sets the linked property status back to "disponible".
- FR-4.5: Validate no overlapping active contracts for the same property.
- FR-4.6: Frontend page with table listing, create/edit form with property and tenant dropdowns.

### FR-5: Payment Tracking (Pagos)
- FR-5.1: CRUD operations for payments via REST API.
- FR-5.2: Payment fields: contrato_id (FK), monto, moneda, fecha_pago, fecha_vencimiento, metodo_pago (efectivo, transferencia, cheque), estado (pendiente, pagado, atrasado), notas.
- FR-5.3: Automatic detection of overdue payments (fecha_vencimiento < today and estado != pagado).
- FR-5.4: Frontend page with table listing and payment recording form.

### FR-6: Dashboard
- FR-6.1: Display summary cards: total properties, occupancy rate, monthly income, overdue payment count.
- FR-6.2: Dashboard data fetched from a dedicated backend endpoint aggregating stats.

## Non-Functional Requirements

### NFR-1: Database
- NFR-1.1: PostgreSQL with SeaORM. UUIDs for all PKs. DECIMAL for monetary values. TIMESTAMPTZ for all dates.
- NFR-1.2: Indexes on all FKs and frequently queried columns (email, cedula, estado, ciudad, provincia).
- NFR-1.3: Migrations implemented via SeaORM MigrationTrait.

### NFR-2: API Design
- NFR-2.1: RESTful JSON API. camelCase response fields via serde rename_all.
- NFR-2.2: Consistent error responses: `{ "error": "type", "message": "description" }`.
- NFR-2.3: Structured logging via tracing crate.

### NFR-3: Frontend
- NFR-3.1: Yew + WASM compiled via Trunk. Tailwind CSS for styling.
- NFR-3.2: SPA with Yew Router. Protected routes redirect to login.
- NFR-3.3: All UI text in Spanish. Dominican locale for dates (DD/MM/YYYY) and currency.

### NFR-4: Security
- NFR-4.1: No plaintext passwords. Argon2 hashing.
- NFR-4.2: JWT-based authentication with configurable secret and expiry.
- NFR-4.3: CORS configured for frontend origin.

## Acceptance Criteria
- AC-1: A user can register, log in, and receive a JWT.
- AC-2: An authenticated gerente can create a property, create a tenant, create a contract linking them, and record a payment against that contract.
- AC-3: A visualizador can view all resources but cannot create, update, or delete.
- AC-4: The dashboard shows accurate aggregate statistics.
- AC-5: Overdue payments are automatically flagged.
- AC-6: All API endpoints return proper error responses for invalid input, unauthorized access, and not-found resources.
- AC-7: The frontend renders all pages with Spanish text and navigates between them via the sidebar.
