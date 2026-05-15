# Tasks: MVP — Real Estate Property Management

## Task 1: Implement database migrations
- [x] Implement `m20250408_000001_create_usuarios` migration with full schema (UUID PK, email unique index, rol check constraint, timestamps)
- [x] Implement `m20250408_000002_create_propiedades` migration with full schema (indexes on ciudad, provincia, tipo_propiedad, estado)
- [x] Implement `m20250408_000003_create_inquilinos` migration with full schema (cedula unique index)
- [x] Implement `m20250408_000004_create_contratos` migration with FKs to propiedades and inquilinos, indexes on FKs and estado
- [x] Implement `m20250408_000005_create_pagos` migration with FK to contratos, indexes on contrato_id, estado, fecha_vencimiento

## Task 2: Implement SeaORM entities
- [x] Implement `backend/src/entities/usuario.rs` — SeaORM entity for usuarios table
- [x] Implement `backend/src/entities/propiedad.rs` — SeaORM entity for propiedades table
- [x] Implement `backend/src/entities/inquilino.rs` — SeaORM entity for inquilinos table
- [x] Implement `backend/src/entities/contrato.rs` — SeaORM entity with relations to propiedad and inquilino
- [x] Implement `backend/src/entities/pago.rs` — SeaORM entity with relation to contrato
- [x] Update `backend/src/entities/prelude.rs` with entity re-exports

## Task 3: Implement request/response models (DTOs)
- [x] Implement `backend/src/models/usuario.rs` — RegisterRequest, LoginRequest, LoginResponse, UserResponse
- [x] Implement `backend/src/models/propiedad.rs` — CreatePropiedadRequest, UpdatePropiedadRequest, PropiedadResponse, PropiedadListQuery
- [x] Implement `backend/src/models/inquilino.rs` — CreateInquilinoRequest, UpdateInquilinoRequest, InquilinoResponse
- [x] Implement `backend/src/models/contrato.rs` — CreateContratoRequest, UpdateContratoRequest, ContratoResponse
- [x] Implement `backend/src/models/pago.rs` — CreatePagoRequest, UpdatePagoRequest, PagoResponse

## Task 4: Implement error handling and config
- [x] Implement `backend/src/errors.rs` — AppError with ResponseError impl returning JSON `{ "error", "message" }` with proper status codes
- [x] Implement `backend/src/config.rs` — load DATABASE_URL, JWT_SECRET, SERVER_PORT from env

## Task 5: Implement auth (JWT + Argon2)
- [x] Implement `backend/src/services/auth.rs` — register (hash password, insert user), login (verify password, issue JWT), JWT encode/decode helpers
- [x] Implement `backend/src/handlers/auth.rs` — POST /api/auth/register, POST /api/auth/login
- [x] Implement `backend/src/middleware/auth.rs` — JWT extraction middleware that sets Claims in request extensions
- [x] Implement `backend/src/middleware/rbac.rs` — role-checking middleware factory

## Task 6: Implement propiedades CRUD
- [x] Implement `backend/src/services/propiedades.rs` — create, get_by_id, list (paginated + filtered), update, delete
- [x] Implement `backend/src/handlers/propiedades.rs` — all 5 REST endpoints with RBAC

## Task 7: Implement inquilinos CRUD
- [x] Implement `backend/src/services/inquilinos.rs` — create, get_by_id, list (searchable), update, delete
- [x] Implement `backend/src/handlers/inquilinos.rs` — all 5 REST endpoints with RBAC

## Task 8: Implement contratos CRUD
- [x] Implement `backend/src/services/contratos.rs` — create (with overlap validation + property status update), update, terminate, list, get_by_id, delete
- [x] Implement `backend/src/handlers/contratos.rs` — all REST endpoints with RBAC

## Task 9: Implement pagos CRUD
- [x] Implement `backend/src/services/pagos.rs` — create, update, list (filterable), get_by_id, delete, overdue detection
- [x] Implement `backend/src/handlers/pagos.rs` — all REST endpoints with RBAC

## Task 10: Implement dashboard endpoint
- [x] Implement `backend/src/services/dashboard.rs` — aggregate stats (total properties, occupancy rate, monthly income, overdue count)
- [x] Implement `backend/src/handlers/dashboard.rs` — GET /api/dashboard/stats

## Task 11: Wire up routes and app entry point
- [x] Implement `backend/src/routes.rs` — register all route groups with RBAC guards
- [x] Implement `backend/src/app.rs` — configure CORS, tracing middleware, JSON config, routes
- [x] Implement `backend/src/main.rs` — init tracing, load config, connect DB, run migrations, start server

## Task 12: Implement frontend types
- [x] Implement `frontend/src/types/usuario.rs` — User, LoginRequest, LoginResponse, RegisterRequest
- [x] Implement `frontend/src/types/propiedad.rs` — Propiedad, CreatePropiedad, UpdatePropiedad
- [x] Implement `frontend/src/types/inquilino.rs` — Inquilino, CreateInquilino, UpdateInquilino
- [x] Implement `frontend/src/types/contrato.rs` — Contrato, CreateContrato, UpdateContrato
- [x] Implement `frontend/src/types/pago.rs` — Pago, CreatePago, UpdatePago

## Task 13: Implement frontend services
- [x] Implement `frontend/src/services/api.rs` — base HTTP helper with JWT attachment and 401 handling
- [x] Implement `frontend/src/services/auth.rs` — login, register, logout, get_token helpers

## Task 14: Implement frontend layout and common components
- [x] Implement `frontend/src/components/layout/navbar.rs` — top nav with user info and logout
- [x] Implement `frontend/src/components/layout/sidebar.rs` — navigation links to all pages
- [x] Implement `frontend/src/components/layout/footer.rs` — simple footer
- [x] Implement `frontend/src/components/common/loading.rs` — spinner component
- [x] Implement `frontend/src/components/common/error_banner.rs` — error display banner
- [x] Implement `frontend/src/components/common/data_table.rs` — reusable table component

## Task 15: Implement frontend auth
- [x] Implement `frontend/src/components/auth/login_form.rs` — login form with validation
- [x] Update `frontend/src/pages/login.rs` — use LoginForm component, handle auth flow
- [x] Implement auth context provider in `frontend/src/app.rs` — JWT storage, protected route redirect

## Task 16: Implement frontend pages
- [x] Implement `frontend/src/pages/dashboard.rs` — fetch and display stats from API
- [x] Implement `frontend/src/pages/propiedades.rs` — table + create/edit form + delete
- [x] Implement `frontend/src/pages/inquilinos.rs` — table + create/edit form + delete
- [x] Implement `frontend/src/pages/contratos.rs` — table + create/edit form with dropdowns + delete
- [x] Implement `frontend/src/pages/pagos.rs` — table + payment recording form

## Task 17: Update frontend app with authenticated layout
- [x] Update `frontend/src/app.rs` — wrap protected routes in layout with sidebar/navbar, add auth context
