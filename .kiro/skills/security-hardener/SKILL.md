---
name: security-hardener
description: >
  Detects and fixes security vulnerabilities in Rust web applications by directly
  editing source files. Hardens authentication, authorization, input validation,
  SQL injection prevention, XSS protection, CORS configuration, secret management,
  and dependency vulnerabilities. Runs cargo audit, cargo clippy, and cargo test
  after changes. Use when hardening security, fixing vulnerabilities, reviewing
  auth flows, or checking for OWASP Top 10 issues.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: project
  version: "1.0.0"
  domain: security
  triggers: security, vulnerability, auth, CORS, XSS, SQL injection, OWASP, hardening, secrets
  role: specialist
  scope: implementation
  output-format: code
  related-skills: perf-optimizer, maintainability-reviewer, code-reviewer
---

# Security Hardener

Specialist skill for detecting and fixing security vulnerabilities in Actix-web + SeaORM + tokio codebases. Actively patches auth flaws, input validation gaps, injection risks, and configuration weaknesses. Validates changes with cargo audit, cargo clippy, and cargo test.

## Core Workflow

1. **Audit dependencies** — Run `cargo audit` to find known vulnerabilities in dependencies
2. **Check authentication** — Verify JWT validation, token expiry, password hashing with argon2, session management
3. **Check authorization** — Verify RBAC middleware enforces role checks on all protected endpoints, no privilege escalation paths
4. **Check input validation** — Verify all user inputs are validated and sanitized before use in queries or responses
5. **Check injection prevention** — Verify SeaORM parameterized queries are used everywhere, no raw SQL, no string interpolation in queries
6. **Check CORS configuration** — Verify CORS is not permissive in production, allowed origins are explicit
7. **Check secret management** — Verify no hardcoded secrets, JWT secrets from env vars, .env not committed
8. **Check error information leakage** — Verify error responses don't expose internal details, stack traces, or database errors to clients
9. **Apply fixes** — Directly edit source files to patch vulnerabilities found

## Reference Guide

| Topic | Reference | Load When |
|-------|-----------|-----------|
| Auth Hardening | `references/auth-hardening.md` | JWT, argon2, session, token rotation |
| Input Validation | `references/input-validation.md` | Sanitization, validation patterns, boundary checks |
| OWASP Rust | `references/owasp-rust.md` | OWASP Top 10 applied to Rust web apps |
| Dependency Audit | `references/dependency-audit.md` | cargo audit, advisory database, update strategies |

## Detection Rules

### Permissive CORS

```rust
// VULNERABLE: allows all origins
.wrap(actix_cors::Cors::permissive())

// FIX: explicit allowed origins
.wrap(
    actix_cors::Cors::default()
        .allowed_origin(&config.frontend_url)
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(3600)
)
```

### Missing Input Validation

```rust
// VULNERABLE: no length check on cedula
pub cedula: String,

// FIX: validate format and length
if cedula.len() != 11 || !cedula.chars().all(|c| c.is_ascii_digit()) {
    return Err(AppError::Validation("Cédula debe tener 11 dígitos".into()));
}
```

### Error Information Leakage

```rust
// VULNERABLE: exposes internal error details
Err(e) => HttpResponse::InternalServerError().json(format!("{:?}", e))

// FIX: generic message, log details internally
Err(e) => {
    tracing::error!("Internal error: {:?}", e);
    HttpResponse::InternalServerError().json(json!({"error": "internal", "message": "Error interno del servidor"}))
}
```

### Hardcoded Secrets

```rust
// VULNERABLE
let secret = "my-super-secret-jwt-key";

// FIX: from environment
let secret = std::env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set");
```

## Constraints

### MUST DO
- Directly edit source files to fix vulnerabilities
- Run `cargo audit` to check dependency vulnerabilities
- Run `cargo fmt` after changes
- Run `cargo clippy --all-targets` to validate changes
- Run `cargo test --workspace` to verify fixes don't break functionality
- If tests fail, revert the breaking change and move on
- Log each fix with file path, vulnerability type, and what was changed
- Use Spanish for any user-facing error messages

### MUST NOT DO
- Suggest fixes without applying them
- Weaken existing security measures
- Remove authentication or authorization checks
- Expose internal error details in responses
- Introduce new dependencies without checking cargo audit first
- Modify generated entity files in backend/src/entities/
