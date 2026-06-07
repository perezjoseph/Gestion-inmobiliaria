# Design Document: External Security Hardening

## Overview

This design addresses 9 external attack vectors identified during a security audit of the production deployment (Cloudflare Tunnel → Caddy → Actix-web backend). Changes span the rate-limiting middleware, auth service, Caddyfile, file upload handlers, container security contexts, and internal service communication.

### Key Design Decisions

1. **Header trust chain**: `CF-Connecting-IP` > `X-Real-Ip` > peer_addr. Cloudflare guarantees CF-Connecting-IP is the real client IP. Traefik sets X-Real-Ip on LAN. X-Forwarded-For is untrusted.
2. **In-memory lockout**: Using a `DashMap<String, LockoutEntry>` in the backend process. Avoids DB writes on every failed login. Acceptable trade-off: lockout state is lost on pod restart (attacker gets a fresh window, but rate limiting still applies).
3. **Magic byte validation**: Validated in the handler before writing to disk. No external dependency needed — just byte slice comparison.
4. **vLLM non-privileged**: Intel GPU access is granted via K8s device plugin resource requests, not container privilege. The device plugin already handles `/dev/dri` device mounting.
5. **JWT revocation via user state**: Instead of maintaining a full revocation list, check `activo` and `password_changed_at` at token validation time. Simple, no cache invalidation complexity.
6. **Internal service auth**: Shared bearer tokens stored in K8s Secrets. Defense-in-depth — NetworkPolicy remains primary, tokens are secondary.

## Architecture

### Request Flow with Security Layers

```
Internet → Cloudflare (WAF + DDoS) → Tunnel → Caddy (body limits, security headers, TLS)
    → Backend (rate limit via CF-Connecting-IP, auth, lockout, RBAC, handlers)
        → Internal Services (bearer token auth + NetworkPolicy)
```

### Component Changes

```
┌─────────────────────────────────────────────────┐
│ Caddy (prod Caddyfile)                          │
│  + Security headers (CSP, HSTS, X-Frame-Options)│
│  + request_body max_size 25MB                   │
│  - Server header removed                        │
└─────────────────────────────────────────────────┘
         │
┌─────────────────────────────────────────────────┐
│ Backend                                         │
│  middleware/rate_limit.rs                        │
│    - CF-Connecting-IP > X-Real-Ip > peer_addr   │
│  services/auth.rs                               │
│    - 8h token expiry, jti + iat claims          │
│    - password_changed_at check in decode_jwt    │
│  services/login_lockout.rs (NEW)                │
│    - DashMap<email, {count, first_failure, locked_until}> │
│    - 5 failures in 15min → 15min lockout        │
│    - Background task prunes expired entries      │
│  handlers/documentos.rs                         │
│    - Magic byte validation before storage       │
│  services/ocr_client.rs                         │
│    - Adds Authorization: Bearer <token>         │
│  services/vllm_client (wherever it lives)       │
│    - Adds api-key header                        │
└─────────────────────────────────────────────────┘
         │
┌─────────────────────────────────────────────────┐
│ Infrastructure                                  │
│  vllm.yml: privileged: true → removed           │
│    + runAsNonRoot, allowPrivilegeEscalation: false│
│  K8s Secrets: ocr-service-token, vllm-api-key   │
└─────────────────────────────────────────────────┘
```

## Detailed Design

### 1. Trusted Client IP Extraction (rate_limit.rs)

Replace `FallbackPeerIpKeyExtractor` logic:

```rust
fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
    let ip = req.headers().get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<IpAddr>().ok())
        .or_else(|| {
            req.headers().get("X-Real-Ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<IpAddr>().ok())
        })
        .unwrap_or_else(|| {
            req.peer_addr().map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), |s| s.ip())
        });
    // IPv6 subnet grouping stays the same
    Ok(normalize_ipv6(ip))
}
```

### 2. Account Lockout (services/login_lockout.rs)

New module with a `LoginLockout` struct:

```rust
pub struct LoginLockout {
    entries: DashMap<String, LockoutEntry>,
}

struct LockoutEntry {
    count: u32,
    first_failure: Instant,
    locked_until: Option<Instant>,
}

impl LoginLockout {
    pub fn check(&self, email: &str) -> Result<(), LockoutError>;  // Returns Err if locked
    pub fn record_failure(&self, email: &str);                      // Increment counter
    pub fn record_success(&self, email: &str);                      // Reset counter
    pub fn cleanup(&self);                                          // Prune expired entries
}
```

Stored as `web::Data<LoginLockout>` in the Actix app. A background `tokio::spawn` task calls `cleanup()` every 5 minutes.

### 3. Production Caddyfile Security Headers

Add to the prod overlay Caddyfile at the top level (applies to all routes):

```caddyfile
header {
    Strict-Transport-Security "max-age=31536000; includeSubDomains"
    X-Content-Type-Options "nosniff"
    X-Frame-Options "DENY"
    Referrer-Policy "strict-origin-when-cross-origin"
    Permissions-Policy "camera=(), microphone=(), geolocation=()"
    Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'"
    X-XSS-Protection "0"
    -Server
}
```

### 4. File Upload Magic Byte Validation

New utility function in `services/file_validation.rs`:

```rust
pub fn validate_magic_bytes(data: &[u8], declared_content_type: &str) -> Result<(), AppError> {
    let valid = match declared_content_type {
        "image/jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "application/pdf" => data.starts_with(b"%PDF"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
        }
        _ => false,
    };
    if !valid {
        return Err(AppError::Validation(
            "El contenido del archivo no coincide con el tipo declarado".into(),
        ));
    }
    Ok(())
}
```

Called in `handlers/documentos.rs` after reading file bytes, before calling the storage service.

### 5. Caddy Request Body Limits

```caddyfile
handle /api/* {
    request_body {
        max_size 25MB
    }
    reverse_proxy backend:8080
}

handle /uploads/* {
    request_body {
        max_size 25MB
    }
    reverse_proxy backend:8080
}
```

### 6. vLLM Privilege Removal

Remove `privileged: true` from the vLLM deployment security context. The Intel GPU device plugin grants access to `/dev/dri` via the resource request `gpu.intel.com/xe: "1"` — no container-level privilege needed.

New security context:
```yaml
securityContext:
  allowPrivilegeEscalation: false
```

Note: `runAsNonRoot` cannot be added because vLLM writes to `/root` (HOME). The `allowPrivilegeEscalation: false` is the key constraint.

### 7. Internal Service Bearer Tokens

**OCR Service**: The Python FastAPI OCR service already supports middleware injection. Add a bearer token check:
- Backend env var: `OCR_SERVICE_TOKEN` (from K8s Secret)
- OCR service env var: `API_TOKEN` (from same Secret)
- Backend sends `Authorization: Bearer <token>` on every OCR request

**vLLM**: vLLM natively supports `--api-key <key>` flag. When set, all requests must include `Authorization: Bearer <key>`.
- Add `--api-key` flag to vLLM args, reading from env var
- Backend sends the key in its vLLM client requests

### 8. JWT Improvements

Changes to `services/auth.rs`:

1. Add `jti: Uuid` and `iat: i64` to the `Claims` struct
2. Reduce expiry from 24h to 8h
3. In `decode_jwt`, after decoding:
   - Query user `activo` status (cache user state if needed)
   - Compare token `iat` against user `password_changed_at`
   - Reject if user inactive or password changed after issuance

Trade-off: This adds a DB query per authenticated request. Mitigation: cache user state in a `DashMap<Uuid, UserSecurityState>` with 60-second TTL.

### 9. Security Event Logging

All events use `tracing::info!` or `tracing::warn!` with structured fields:

```rust
tracing::warn!(
    event = "login_failed",
    email = %email,
    client_ip = %ip,
    "Failed login attempt"
);

tracing::warn!(
    event = "account_locked",
    email = %email,
    client_ip = %ip,
    locked_until = %until,
    "Account locked due to repeated failures"
);
```

## Migration Requirements

### Database Migration

Add `password_changed_at TIMESTAMPTZ` column to `usuarios` table:
- Default: `created_at` value (so existing tokens remain valid)
- Updated whenever password is changed

### Kubernetes Secrets

New secrets to create:
- `realestate-internal-service-tokens` with keys: `ocr-service-token`, `vllm-api-key`

### Deployment Coordination

1. Deploy OCR service with token validation first (accepting both auth and no-auth during transition)
2. Deploy backend with new token sending
3. Remove no-auth fallback from OCR service

For vLLM, the `--api-key` flag can be added atomically since only the backend calls it.

## Dependencies

- `dashmap` crate — concurrent HashMap for lockout state. Already widely used, actively maintained.
- No other new dependencies required.

## Components and Interfaces

### Modified Components

| Component | File(s) | Change |
|-----------|---------|--------|
| Rate Limit Extractor | `backend/src/middleware/rate_limit.rs` | Replace X-Forwarded-For with CF-Connecting-IP > X-Real-Ip chain |
| Auth Service | `backend/src/services/auth.rs` | Add jti/iat claims, 8h expiry, password_changed_at check |
| Login Handler | `backend/src/handlers/auth.rs` | Integrate lockout check before and after login |
| Document Handler | `backend/src/handlers/documentos.rs` | Add magic byte validation call |
| OCR Client | `backend/src/services/ocr_client.rs` | Add Authorization header |
| App Setup | `backend/src/app.rs` | Register LoginLockout as app_data, spawn cleanup task |
| Config | `backend/src/config.rs` | Add `ocr_service_token` and `vllm_api_key` fields |

### New Components

| Component | File(s) | Purpose |
|-----------|---------|---------|
| Login Lockout | `backend/src/services/login_lockout.rs` | In-memory per-email lockout tracking |
| File Validation | `backend/src/services/file_validation.rs` | Magic byte content-type verification |
| User Security Cache | `backend/src/services/user_security_cache.rs` | Cached activo + password_changed_at for JWT validation |

### Infrastructure Components

| Component | File(s) | Change |
|-----------|---------|--------|
| Prod Caddyfile | `infra/k8s/app/overlays/prod/Caddyfile` | Add security headers + body size limits |
| vLLM Deployment | `infra/k8s/app/shared/vllm.yml` | Remove privileged: true |
| K8s Secrets | Manual creation | `realestate-internal-service-tokens` |

### Interfaces

**LoginLockout API:**
```rust
impl LoginLockout {
    pub fn new() -> Self;
    pub fn check(&self, email: &str) -> Result<(), LockoutInfo>;
    pub fn record_failure(&self, email: &str) -> Option<LockoutInfo>;
    pub fn record_success(&self, email: &str);
    pub fn cleanup(&self);
}

pub struct LockoutInfo {
    pub retry_after_seconds: u64,
}
```

**File Validation API:**
```rust
pub fn validate_magic_bytes(data: &[u8], content_type: &str) -> Result<(), AppError>;
```

**User Security Cache API:**
```rust
impl UserSecurityCache {
    pub fn new() -> Self;
    pub async fn is_token_valid(&self, db: &DatabaseConnection, user_id: Uuid, iat: i64) -> Result<bool, AppError>;
    pub fn invalidate(&self, user_id: Uuid);
}
```

## Data Models

### LockoutEntry (in-memory only)

```rust
struct LockoutEntry {
    count: u32,
    first_failure: Instant,
    locked_until: Option<Instant>,
}
```

### Extended JWT Claims

```rust
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub rol: String,
    pub organizacion_id: Uuid,
    pub jti: Uuid,    // NEW: unique token ID
    pub iat: i64,     // NEW: issued-at timestamp
    pub exp: usize,
}
```

### Database Migration: usuarios table

```sql
ALTER TABLE usuarios ADD COLUMN password_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
UPDATE usuarios SET password_changed_at = created_at;
```

### UserSecurityState (cached)

```rust
struct UserSecurityState {
    activo: bool,
    password_changed_at: DateTime<Utc>,
    cached_at: Instant,
}
```

## Correctness Properties

### Property 1: Rate Limit IP Attribution

Every external request MUST be attributed to a real client IP. If all trusted headers are absent, the system falls back to peer_addr — never to a default/shared IP that would group unrelated clients.

**Validates: Requirements 1.1, 1.2, 1.3, 1.4**

### Property 2: Lockout Monotonicity

Once an account is locked, it stays locked for the full 15-minute window. Concurrent requests cannot reset the counter by racing.

**Validates: Requirements 2.2, 2.3**

### Property 3: Token Invalidity Propagation

Deactivating a user or changing a password takes effect within the cache TTL (60 seconds max). No token issued before the security event can be used after the cache refreshes.

**Validates: Requirements 8.3, 8.4**

### Property 4: Magic Byte Non-Bypassability

The magic byte check occurs after file data is fully read into memory and before any storage or processing call. No code path can skip the validation.

**Validates: Requirements 4.1, 4.3**

### Property 5: Defense-in-Depth Body Limits

Both Caddy (25MB) and the backend (20MB multipart, 1MB JSON) enforce limits. The tighter backend limit is the effective one for valid requests; the Caddy limit catches malformed/oversized payloads before they consume backend resources.

**Validates: Requirements 5.1, 5.2**

## Error Handling

| Scenario | Response | HTTP Status |
|----------|----------|-------------|
| Account locked | `{"error": "account_locked", "retry_after_seconds": N}` | 429 |
| Magic bytes mismatch | `{"error": "validation", "message": "El contenido del archivo no coincide con el tipo declarado"}` | 400 |
| Request body too large (Caddy) | Caddy default 413 response | 413 |
| Token for deactivated user | `{"error": "unauthorized", "message": "Sesión inválida"}` | 401 |
| Token issued before password change | `{"error": "unauthorized", "message": "Sesión inválida"}` | 401 |
| Missing internal service token | Service returns 401 (OCR) or 403 (vLLM) | Backend maps to 503 for client |

## Testing Strategy

1. **Unit Tests** (backend):
   - `login_lockout.rs`: Test counter increment, lockout trigger at threshold, reset on success, expiry after 15 min, cleanup of stale entries.
   - `file_validation.rs`: Test each magic byte pattern (JPEG, PNG, PDF, DOCX), rejection of mismatched bytes, empty file handling.
   - `rate_limit.rs`: Test header priority (CF-Connecting-IP > X-Real-Ip > peer_addr), IPv6 normalization.
   - `auth.rs`: Test jti/iat claim generation, token rejection when iat < password_changed_at.

2. **Integration Tests** (backend):
   - Login lockout flow: 5 failures → lockout → wait → unlock.
   - File upload with invalid magic bytes → 400.
   - Deactivated user token → 401.

3. **Infrastructure Tests**:
   - Verify security headers present in Caddy response (`curl -I`).
   - Verify vLLM starts and serves inference without privileged mode.
   - Verify OCR service rejects requests without bearer token.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Lockout state lost on pod restart | Attacker gets fresh 5 attempts | Rate limiting still applies (10 req burst); brief window acceptable |
| JWT decode now checks DB (user state) | Latency increase per request | Cache user security state with 60s TTL in DashMap |
| vLLM without privileged flag may fail | AI inference unavailable | Test thoroughly; Intel device plugin handles device access |
| OCR token deployment ordering | Brief OCR outage if misconfigured | Deploy with dual-mode (accept both) during transition |
