# Requirements Document

## Introduction

This feature hardens the platform against external attacks identified during a security audit. The application is exposed to the internet via a Cloudflare Tunnel and serves a property management SPA (Caddy frontend) with a Rust/Actix-web backend API. The audit identified vulnerabilities in rate limiting, missing security headers in production, lack of account lockout, IP trust chain issues, and insufficient file upload validation. All fixes target the external attack surface — no internal architecture changes.

## Glossary

- **CF-Connecting-IP**: HTTP header set by Cloudflare containing the true client IP. Cannot be spoofed by end users since Cloudflare overwrites it.
- **X-Real-Ip**: Header set by Traefik for local ingress containing the client IP from the LAN.
- **Account Lockout**: Temporary disabling of login attempts for an account after repeated failures, to prevent brute-force attacks.
- **Security Headers**: HTTP response headers that instruct browsers to apply security policies (CSP, HSTS, X-Frame-Options, etc.).
- **Magic Bytes**: The first bytes of a file that identify its format (e.g., `%PDF`, `\xFF\xD8\xFF` for JPEG). Used to validate file content independently of the declared extension or MIME type.
- **Trusted Proxy Header**: The specific HTTP header used to determine the real client IP based on the reverse proxy in front of the application.

## Requirements

### Requirement 1: Trusted Client IP Extraction

**User Story:** As a platform operator, I want the backend to correctly identify client IPs regardless of whether traffic arrives via Cloudflare (production) or Traefik (LAN), so that rate limiting and audit logs use the real client IP.

#### Acceptance Criteria

1. WHEN the request contains a `CF-Connecting-IP` header, THE rate limiter SHALL use that IP as the client identity.
2. WHEN the request does NOT contain `CF-Connecting-IP` but contains `X-Real-Ip`, THE rate limiter SHALL use the `X-Real-Ip` header value.
3. WHEN neither trusted header is present, THE rate limiter SHALL fall back to the socket peer address.
4. THE rate limiter SHALL NOT trust `X-Forwarded-For` directly, as it can be spoofed by clients.
5. THE extracted IP SHALL be used consistently for both rate limiting and login audit logging.

### Requirement 2: Account Lockout on Failed Login

**User Story:** As a platform operator, I want accounts to be temporarily locked after repeated failed login attempts, so that brute-force and credential-stuffing attacks are mitigated.

#### Acceptance Criteria

1. WHEN a login attempt fails, THE system SHALL increment a per-email failure counter.
2. WHEN the failure counter for an email reaches 5 within a 15-minute window, THE system SHALL reject further login attempts for that email for 15 minutes, regardless of IP.
3. WHEN the lockout period expires, THE failure counter SHALL reset and login attempts SHALL be allowed again.
4. WHEN a successful login occurs before lockout, THE failure counter for that email SHALL reset to zero.
5. THE lockout response SHALL return HTTP 429 with a JSON body containing `"error": "account_locked"` and a `"retry_after_seconds"` field.
6. THE lockout state SHALL be stored in-memory (not DB) to avoid adding write load on every login attempt.
7. THE lockout mechanism SHALL NOT reveal whether an email exists in the system — locked and non-existent emails SHALL return the same generic unauthorized response until lockout threshold is met.

### Requirement 3: Production Security Headers in Caddy

**User Story:** As a platform operator, I want the production Caddyfile to include all security headers, so that browsers enforce CSP, clickjacking protection, and HSTS for end users.

#### Acceptance Criteria

1. THE production Caddyfile SHALL include the following headers on all responses: `Strict-Transport-Security`, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`, `Content-Security-Policy`, `X-XSS-Protection`.
2. THE `Content-Security-Policy` SHALL allow `'self'` for default-src, scripts, and styles (`'unsafe-inline'` for styles only), `data:` and `blob:` for images, and `'wasm-unsafe-eval'` for script-src (required by the Yew/WASM frontend).
3. THE `Server` header SHALL be removed from responses to avoid fingerprinting.
4. THE `X-Frame-Options` SHALL be set to `DENY` to prevent clickjacking.
5. THE security headers SHALL match the existing base Caddyfile configuration.

### Requirement 4: File Upload Content Validation

**User Story:** As a platform operator, I want uploaded files to be validated by magic bytes (not just extension), so that attackers cannot upload executable or malicious files disguised with valid extensions.

#### Acceptance Criteria

1. WHEN a file is uploaded via the documents endpoint, THE handler SHALL validate the first bytes of the file against known magic bytes for the declared content type.
2. THE system SHALL accept only files whose magic bytes match: JPEG (`FF D8 FF`), PNG (`89 50 4E 47`), PDF (`25 50 44 46`), DOCX/XLSX (ZIP signature `50 4B 03 04`).
3. IF the magic bytes do not match the declared content type, THE handler SHALL reject the upload with HTTP 400 and a clear error message.
4. THE existing extension-based validation SHALL remain as a first-pass filter, with magic byte validation as a second layer.

### Requirement 5: Request Body Size Limits at Caddy

**User Story:** As a platform operator, I want Caddy to enforce maximum request body sizes before proxying to the backend, so that oversized requests don't consume backend memory.

#### Acceptance Criteria

1. THE production Caddyfile SHALL limit request bodies to 25 MB for `/uploads/*` and `/api/*` routes.
2. IF a request exceeds the limit, Caddy SHALL return HTTP 413 (Request Entity Too Large) without proxying to the backend.

### Requirement 6: Remove vLLM Privileged Mode

**User Story:** As a platform operator, I want the vLLM container to run without full privileged mode, so that a container escape cannot grant host-level access.

#### Acceptance Criteria

1. THE vLLM deployment SHALL NOT use `privileged: true` in the security context.
2. THE vLLM deployment SHALL explicitly grant only the device access needed for Intel GPU via resource requests (`gpu.intel.com/xe`), without broad privilege escalation.
3. THE vLLM container SHALL continue to function correctly for inference after the change.

### Requirement 7: Internal Service Authentication

**User Story:** As a platform operator, I want internal services (OCR, vLLM) to require a shared secret token on requests, so that if network policies are accidentally removed, the services are still protected.

#### Acceptance Criteria

1. THE backend SHALL send an `Authorization: Bearer <token>` header when calling the OCR service.
2. THE OCR service SHALL reject requests that do not include a valid bearer token with HTTP 401.
3. THE shared token SHALL be stored in a Kubernetes Secret and injected via environment variable.
4. THE vLLM service SHALL require an `--api-key` flag matching a shared secret for all inference requests.
5. THE backend's vLLM client SHALL include the API key in all requests.

### Requirement 8: JWT Token Improvements

**User Story:** As a platform operator, I want JWT tokens to have shorter lifetimes and support forced revocation, so that compromised tokens have limited blast radius.

#### Acceptance Criteria

1. THE JWT token expiry SHALL be reduced from 24 hours to 8 hours.
2. THE system SHALL maintain an in-memory revocation set of JTI (JWT ID) values.
3. WHEN an admin deactivates a user (sets `activo = false`), THE auth middleware SHALL reject tokens for that user even if not expired.
4. WHEN a user changes their password, ALL existing tokens for that user SHALL become invalid (via a `password_changed_at` timestamp compared against the token's `iat`).
5. Each JWT SHALL include a `jti` (UUID) and `iat` (issued-at) claim.

### Requirement 9: Audit Logging for Security Events

**User Story:** As a platform operator, I want all security-relevant events logged with structured metadata, so that incidents can be investigated.

#### Acceptance Criteria

1. THE system SHALL log the following events at INFO level: successful login, failed login (with IP and email), account lockout triggered, account lockout expired, user deactivated, password changed.
2. Each security log entry SHALL include: timestamp, event type, client IP, user email (when available), and user ID (when available).
3. THE login failure log SHALL NOT include the attempted password.
