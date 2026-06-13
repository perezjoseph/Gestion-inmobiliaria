---
inclusion: always
---

# Project Structure

Rust 2024 workspace (members: `backend`, `frontend`). Android: Kotlin + Compose.

## Layout

- `backend/` — Actix-web API (see `backend.md` when editing)
- `frontend/` — Leptos SPA (see conditional steering when editing)
- `android/` — Kotlin + Compose mobile app (see `android.md` when editing)
- `infra/` — Kubernetes manifests, Dockerfiles
- `baileys-service/` — WhatsApp sidecar (Node/TS)
- `ocr-service/` — OCR microservice

## Infrastructure (`infra/`)

Deployed on Kubernetes (k3s). Manifests in `infra/k8s/app/`. Services: backend, frontend (Caddy), baileys (WhatsApp sidecar), ovms (AI inference), postgres. Env vars and secrets via K8s Secrets. `docker-compose.dev.yml` exists for local DB only — never reference it for deployment or runtime config. All debugging of inter-service communication should assume K8s networking (service DNS names, not localhost).

## Naming

Rust `snake_case`, Kotlin `PascalCase`. New domain workflow: migration -> entity -> DTOs -> service -> handler -> routes -> tests.
