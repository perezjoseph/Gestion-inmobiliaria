---
inclusion: always
---

# Project Structure

- Always place new files in the correct location according to this structure.
- Never create files outside this layout without explicit instruction.

## Root
- `AGENTS.md` — agent rules
- `product.md` — product description
- `lessons-learned.md` — lessons learned log
- `Cargo.toml` — workspace manifest
- `docker-compose.yml` — local services
- `.env.example` — environment template

## Backend (`backend/`)
- `backend/src/main.rs` — entry point
- `backend/src/config.rs` — configuration
- `backend/src/app.rs` — app builder
- `backend/src/errors.rs` — error types
- `backend/src/routes.rs` — route registration
- `backend/src/middleware/` — auth.rs, rbac.rs
- `backend/src/handlers/` — auth.rs, propiedades.rs, inquilinos.rs, contratos.rs, pagos.rs
- `backend/src/services/` — auth.rs, propiedades.rs, inquilinos.rs, contratos.rs, pagos.rs
- `backend/src/models/` — usuario.rs, propiedad.rs, inquilino.rs, contrato.rs, pago.rs
- `backend/src/entities/` — SeaORM generated entities
- `backend/migrations/` — SeaORM migration files
- `backend/tests/` — integration tests per domain

## Frontend (`frontend/`)
- `frontend/src/main.rs` — entry point
- `frontend/src/app.rs` — app component and routes
- `frontend/src/components/layout/` — navbar.rs, sidebar.rs, footer.rs
- `frontend/src/components/common/` — loading.rs, error_banner.rs, data_table.rs
- `frontend/src/components/auth/` — login_form.rs
- `frontend/src/components/<feature>/` — propiedades, inquilinos, contratos, pagos
- `frontend/src/pages/` — login.rs, dashboard.rs, propiedades.rs, inquilinos.rs, contratos.rs, pagos.rs
- `frontend/src/services/` — api.rs, auth.rs
- `frontend/src/types/` — usuario.rs, propiedad.rs, inquilino.rs, contrato.rs, pago.rs
- `frontend/tests/` — integration_tests.rs

## Tailwind Tooling (root)
- `package.json` — Tailwind CSS and Node dev dependencies
- `node_modules/` — auto-generated, never committed

## Scripts (`scripts/`)
- `scripts/setup.sh` — project setup
- `scripts/migrate.sh` — database migrations
