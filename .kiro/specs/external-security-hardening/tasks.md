# Implementation Plan: External Security Hardening

## Overview

This plan addresses 9 external attack vectors identified during a security audit. Tasks are ordered by dependency and risk priority: IP trust chain and lockout first (highest impact), then headers/body limits (quick wins), then file validation, JWT improvements, internal auth, and finally vLLM privilege removal.

## Tasks

- [x] 1. Fix trusted client IP extraction in rate limiter
  - [x] 1.1 Update `FallbackPeerIpKeyExtractor` in `backend/src/middleware/rate_limit.rs`
    - Replace X-Forwarded-For trust with: CF-Connecting-IP → X-Real-Ip → peer_addr priority chain
    - Keep existing IPv6 /56 subnet normalization
    - Add unit tests for header priority logic
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 2. Implement account lockout on failed login
  - [x] 2.1 Add `dashmap` dependency to `backend/Cargo.toml`
    - Research latest stable version and pin it
    - _Requirements: 2.6_

  - [x] 2.2 Create `backend/src/services/login_lockout.rs`
    - Implement `LoginLockout` struct with `DashMap<String, LockoutEntry>`
    - `check(email)`: return error if currently locked
    - `record_failure(email)`: increment counter, trigger lockout at 5 failures within 15 min
    - `record_success(email)`: reset counter
    - `cleanup()`: remove entries older than 15 minutes
    - Re-export in `backend/src/services/mod.rs`
    - Add unit tests: counter increment, lockout at threshold, reset on success, expiry
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

  - [x] 2.3 Integrate lockout into auth handler and app setup
    - Register `LoginLockout` as `web::Data` in `backend/src/app.rs`
    - Spawn background cleanup task (every 5 minutes) in app startup
    - In `backend/src/handlers/auth.rs` `login()`: call `lockout.check()` before attempting login, call `record_failure()` on failure, `record_success()` on success
    - Return HTTP 429 with `{"error": "account_locked", "retry_after_seconds": N}` when locked
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.7_

  - [x] 2.4 Add security event logging for lockout
    - Log `event=login_failed` with email + client_ip on every failed attempt
    - Log `event=account_locked` with email + client_ip + locked_until when lockout triggers
    - Ensure attempted password is NOT logged
    - _Requirements: 9.1, 9.2, 9.3_

- [x] 3. Add security headers and body size limits to production Caddyfile
  - [x] 3.1 Update `infra/k8s/app/overlays/prod/Caddyfile`
    - Add global `header` block with: HSTS, X-Content-Type-Options, X-Frame-Options DENY, Referrer-Policy, Permissions-Policy, CSP (with wasm-unsafe-eval for Yew), X-XSS-Protection 0, -Server
    - Add `request_body { max_size 25MB }` to `/api/*` and `/uploads/*` handlers
    - Match the base Caddyfile security headers
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 5.1, 5.2_

- [x] 4. Checkpoint — verify rate limiter, lockout, and Caddy changes compile/work
  - Build backend with `cargo check`
  - Run rate_limit and login_lockout unit tests
  - Verify Caddyfile syntax is valid (caddy fmt or caddy validate)

- [x] 5. Implement file upload magic byte validation
  - [x] 5.1 Create `backend/src/services/file_validation.rs`
    - Implement `validate_magic_bytes(data: &[u8], content_type: &str) -> Result<(), AppError>`
    - Support: JPEG (FF D8 FF), PNG (89 50 4E 47), PDF (%PDF), DOCX/XLSX (PK 50 4B 03 04)
    - Return AppError::Validation with clear Spanish message on mismatch
    - Re-export in `backend/src/services/mod.rs`
    - Add unit tests for each file type + rejection cases
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [x] 5.2 Integrate magic byte validation into document upload handler
    - In `backend/src/handlers/documentos.rs` `upload()`: call `validate_magic_bytes` after reading file bytes, before calling storage service
    - Determine content type from extension (existing logic), then validate bytes match
    - _Requirements: 4.1, 4.3_

  - [x] 5.3 Integrate magic byte validation into OCR handler
    - In `backend/src/handlers/ocr.rs` `ocr_extract()`: call `validate_magic_bytes` after reading file data
    - _Requirements: 4.1, 4.3_

- [ ] 6. JWT token improvements
  - [x] 6.1 Create database migration for `password_changed_at`
    - Create `m{date}_000001_add_password_changed_at.rs`
    - Add `password_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()` to `usuarios`
    - Backfill existing rows: `SET password_changed_at = created_at`
    - _Requirements: 8.4_

  - [x] 6.2 Update usuario entity with `password_changed_at` field
    - Add `password_changed_at: DateTimeWithTimeZone` to `backend/src/entities/usuario.rs`
    - _Requirements: 8.4_

  - [x] 6.3 Create `backend/src/services/user_security_cache.rs`
    - Implement `UserSecurityCache` with `DashMap<Uuid, UserSecurityState>`
    - `is_token_valid(db, user_id, iat)`: check user activo + password_changed_at vs iat, with 60s cache TTL
    - `invalidate(user_id)`: remove from cache (called on deactivation/password change)
    - Re-export in `backend/src/services/mod.rs`
    - _Requirements: 8.3, 8.4_

  - [x] 6.4 Update JWT claims and auth service
    - Add `jti: Uuid` and `iat: i64` to `Claims` struct in `backend/src/services/auth.rs`
    - Set `jti = Uuid::new_v4()` and `iat = Utc::now().timestamp()` in `encode_jwt`
    - Reduce token expiry from 24 hours to 8 hours
    - Update `decode_jwt` validation to require `jti` and `iat` claims
    - _Requirements: 8.1, 8.5_

  - [x] 6.5 Integrate user security cache into auth middleware
    - Register `UserSecurityCache` as `web::Data` in `backend/src/app.rs`
    - In auth middleware (`backend/src/middleware/auth.rs`), after decoding JWT: call `cache.is_token_valid(db, claims.sub, claims.iat)`
    - Return 401 if user is inactive or password changed after token issuance
    - _Requirements: 8.3, 8.4_

  - [~] 6.6 Invalidate cache on password change and user deactivation
    - In password change service: update `password_changed_at` in DB, call `cache.invalidate(user_id)`
    - In user deactivation service: call `cache.invalidate(user_id)` after setting `activo = false`
    - Log `event=password_changed` and `event=user_deactivated` with user_id
    - _Requirements: 8.3, 8.4, 9.1_

- [~] 7. Checkpoint — verify JWT changes compile and tests pass
  - `cargo check` and run existing auth tests
  - Verify new migration applies cleanly

- [ ] 8. Internal service authentication
  - [~] 8.1 Add OCR service bearer token to backend
    - Add `ocr_service_token: Option<String>` to `AppConfig` (from `OCR_SERVICE_TOKEN` env var)
    - In `OcrClient::new()`: read token from env
    - In `OcrClient::extract()`: add `Authorization: Bearer <token>` header when token is configured
    - _Requirements: 7.1, 7.3_

  - [~] 8.2 Add vLLM API key to backend
    - Add `vllm_api_key: Option<String>` to `ChatbotEnvConfig` (from `VLLM_API_KEY` env var)
    - In vLLM client requests: add `Authorization: Bearer <key>` header when configured
    - _Requirements: 7.4, 7.5_

  - [~] 8.3 Update K8s deployment manifests for internal service tokens
    - Add `OCR_SERVICE_TOKEN` env var to backend deployment (from Secret)
    - Add `VLLM_API_KEY` env var to backend deployment (from Secret)
    - Add `--api-key` argument to vLLM deployment args (from env var)
    - Document Secret creation in cloudflare-tunnel.yml comment pattern
    - _Requirements: 7.3, 7.4_

- [ ] 9. Remove vLLM privileged mode
  - [~] 9.1 Update `infra/k8s/app/shared/vllm.yml` security context
    - Remove `privileged: true`
    - Add `allowPrivilegeEscalation: false`
    - Keep GPU resource request (`gpu.intel.com/xe: "1"`) which handles device access
    - _Requirements: 6.1, 6.2, 6.3_

- [~] 10. Final verification
  - Run `cargo build` for full compilation check
  - Run `cargo test` for all backend tests
  - Run `cargo clippy` for lint check
  - Verify K8s manifests are valid YAML
  - Verify Caddyfile syntax

## Task Dependency Graph

```json
{
  "waves": [
    {
      "name": "Wave 1 — Rate limiting, lockout, and Caddy hardening",
      "tasks": [1, 2, 3],
      "description": "Independent tasks that fix the highest-priority external vulnerabilities"
    },
    {
      "name": "Wave 2 — Checkpoint 1",
      "tasks": [4],
      "description": "Verify rate limiter, lockout, and Caddyfile compile and work"
    },
    {
      "name": "Wave 3 — File validation and JWT improvements",
      "tasks": [5, 6],
      "description": "Independent tasks for upload hardening and token lifecycle"
    },
    {
      "name": "Wave 4 — Checkpoint 2",
      "tasks": [7],
      "description": "Verify JWT changes and file validation compile and tests pass"
    },
    {
      "name": "Wave 5 — Internal auth and privilege removal",
      "tasks": [8, 9],
      "description": "Defense-in-depth for internal services and container security"
    },
    {
      "name": "Wave 6 — Final verification",
      "tasks": [10],
      "description": "Full build, test, and lint pass"
    }
  ]
}
```

## Notes

- The `dashmap` crate version must be researched before adding (per project dependency rules).
- The OCR service (Python) needs a corresponding change to validate the bearer token. That change is out of scope for this spec but should be tracked as a follow-up task.
- vLLM privilege removal should be tested on the inference node before merging. If GPU access breaks, the `privileged: true` can be temporarily restored while investigating Intel device plugin configuration.
- JWT claim changes are backward-incompatible: existing tokens will fail validation after deployment. Coordinate deployment during low-traffic hours or implement a grace period that accepts tokens without `jti`/`iat` for 8 hours after deploy.
