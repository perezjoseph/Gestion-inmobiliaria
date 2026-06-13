---
inclusion: always
---

# Project Structure

Rust 2024 workspace (members: `backend`, `frontend`). Android: Kotlin + Compose.

## Backend (`backend/src/`)

`main.rs, app.rs, config.rs, errors.rs, lib.rs, routes.rs` | `middleware/` (auth, rbac) | `handlers/` | `services/` | `models/` (DTOs) | `entities/` (generated) | `migrations/` | `tests/`

Domains: auth, propiedades, inquilinos, contratos, pagos, gastos, mantenimiento, dashboard, auditoria, usuarios, perfil, notificaciones, reportes, recibos, documentos, configuracion, importacion.

New domain: migration -> entity -> DTOs -> service -> handler -> routes -> tests. Re-export in each `mod.rs`.

## Frontend (`frontend/src/`)

`main.rs, app.rs, lib.rs, utils.rs` | `components/{layout,common,feature}/` | `pages/` | `services/` | `types/`

New feature: types -> api calls -> components -> page -> route. Re-export in each `mod.rs`.

## Android (`android/`)

`app/` | `core/{common,data,database,model,network,ui}` | `feature/{domain}/`

Naming: Rust `snake_case`, Kotlin `PascalCase`. Migration and test-file naming: see `backend.md` and `testing.md`.

## Infrastructure (`infra/`)

Deployed on Kubernetes (k3s). Manifests in `infra/k8s/app/`. Services: backend, frontend (Caddy), baileys (WhatsApp sidecar), ovms (AI inference), postgres. Env vars and secrets via K8s Secrets. `docker-compose.dev.yml` exists for local DB only — never reference it for deployment or runtime config. All debugging of inter-service communication should assume K8s networking (service DNS names, not localhost).
