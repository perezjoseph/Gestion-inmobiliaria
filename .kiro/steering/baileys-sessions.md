---
inclusion: fileMatch
fileMatchPattern: ["baileys-service/**/*", "backend/migrations/**/*whatsapp*", "infra/k8s/app/baileys.yml"]
---

# Baileys Session Persistence

## Architecture Decision: Direct DB Access with Isolation

The baileys-service sidecar stores WhatsApp auth state (Signal Protocol keys) directly in PostgreSQL. This is intentional — not a shared-database antipattern — because:

1. Signal Protocol calls `keys.set` on **every message** (session ratchet). HTTP proxy adds 1-5ms per write, risking message backlog.
2. `SignalKeyStoreWithTransaction` requires real DB transactions. HTTP APIs can't expose this cleanly.
3. Every community Baileys DB store (postgres-baileys, supabase-baileys, baileys-store/Prisma) uses direct access.

## Isolation Guardrails

- **Dedicated PG role**: `whatsapp_session_rw` with `GRANT` only on `whatsapp_auth_*` tables. No access to business tables.
- **Table prefix**: All auth tables use `whatsapp_auth_` prefix (e.g. `whatsapp_auth_creds`, `whatsapp_auth_keys`).
- **Migrations owned by backend**: Single migration source of truth. The sidecar never runs migrations.
- **Cache wrapper**: Always use `makeCacheableSignalKeyStore` to minimize DB round-trips.
- **Encryption at rest**: All stored values are AES-256-GCM encrypted at the application layer before DB write.

## Tables

- `whatsapp_auth_creds(realm_id PK, creds_data BYTEA, updated_at TIMESTAMPTZ)` — device identity per org
- `whatsapp_auth_keys(realm_id, category, key_id, key_data BYTEA, updated_at TIMESTAMPTZ)` — Signal key-value store, PK `(realm_id, category, key_id)`

## Rules

- Never give the sidecar's PG role access beyond `whatsapp_auth_*` tables.
- Never store keys unencrypted. Always encrypt with `SESSION_ENCRYPTION_KEY` before DB write.
- Never skip `makeCacheableSignalKeyStore` — without it, every message triggers a DB read.
- On pod startup, scan `whatsapp_auth_creds` and call `startSession` for each realm to restore connections.
- The `emptyDir` sessions volume in K8s is removed — no filesystem session state.
