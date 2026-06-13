# Security Checklist

Detailed checklists for different stages of development. Use before merging PRs, before deployments, and during periodic security reviews.

## Pre-Merge Checklist (Every PR)

### Authentication & Authorization

- [ ] Every new endpoint uses the correct extractor:
  - Read-only: `Claims` (any authenticated user)
  - Mutations: `WriteAccess` (admin or gerente)
  - User management: `AdminOnly` (admin only)
- [ ] No endpoint is accidentally public (missing auth extractor)
- [ ] Token expiration is ≤ 30 minutes for access tokens
- [ ] Password changes invalidate existing sessions/tokens
- [ ] Failed login attempts are logged with `tracing::warn!`

### Multi-Tenant Isolation

- [ ] Every database query filters by `organizacion_id` from Claims
- [ ] No endpoint allows accessing resources from another organization
- [ ] Bulk operations (list, export) are scoped to the user's org
- [ ] Related entity lookups verify parent belongs to same org
  - e.g., creating a pago verifies the contrato belongs to the user's org

### Input Validation

- [ ] All string inputs validated for max length (match DB VARCHAR constraints)
- [ ] Required fields checked for non-empty after `.trim()`
- [ ] Email fields validated with regex or dedicated validator
- [ ] Monetary amounts: positive, ≤ 2 decimal places
- [ ] Date ranges: start < end, no dates in unreasonable past/future
- [ ] Enum fields: validated against allowed values (not just serde deserialization)
- [ ] Path parameters: UUID format enforced by `web::Path<Uuid>`
- [ ] Request body size limited (JsonConfig::limit — currently 1MB)

### Error Handling

- [ ] No internal details leaked in error responses
- [ ] `AppError::Internal` used for unexpected errors (hides message)
- [ ] `AppError::Validation` used for user-fixable input errors (shows message)
- [ ] Database errors mapped to `Internal`, not `BadRequest`
- [ ] Detailed errors logged server-side with `tracing::error!`

### Data Safety

- [ ] No `password_hash` field in any API response DTO
- [ ] No tokens or secrets in log output
- [ ] Sensitive fields excluded from `#[derive(Debug)]` or use custom Debug impl
- [ ] File paths sanitized (no `..`, `/`, `\` in user-provided filenames)

### Dependencies

- [ ] `cargo deny check` passes
- [ ] No new `unsafe` blocks in production code
- [ ] New dependencies are well-maintained (check last publish date, downloads)
- [ ] New dependencies have compatible licenses

---

## Pre-Deployment Checklist

### Configuration

- [ ] `CORS_ORIGIN` set to exact production frontend URL (not empty/permissive)
- [ ] `JWT_SECRET` is ≥ 32 characters, randomly generated, unique per environment
- [ ] `DATABASE_URL` uses `sslmode=require` for production
- [ ] `SERVER_PORT` is behind a reverse proxy (not directly exposed)
- [ ] No `.env` file deployed — secrets come from secrets manager or env injection
- [ ] `UPLOAD_DIR` points to a non-web-accessible directory

### Infrastructure

- [ ] TLS termination configured (nginx/ALB/CloudFront in front)
- [ ] Security headers middleware active (DefaultHeaders)
- [ ] Rate limiting active on auth endpoints
- [ ] Database user has minimal privileges (no CREATE/DROP for app user)
- [ ] Migrations run with a separate elevated DB user
- [ ] Logs don't contain PII or secrets
- [ ] Health endpoint (`/health`) doesn't expose version or internal state
- [ ] K8s pods drop ALL capabilities and use seccomp RuntimeDefault
- [ ] NetworkPolicies enforce default-deny (only backend → db, frontend → backend)
- [ ] Pod Security Admission set to `restricted` on namespace
- [ ] PostgreSQL image pinned by digest in K8s manifests
- [ ] `/uploads` static file serving replaced with authenticated handler

### Monitoring

- [ ] Failed auth attempts generate alerts above threshold
- [ ] 5xx error rate monitored
- [ ] Rate limit hits (429s) monitored for DDoS detection
- [ ] Disk usage monitored (upload directory, logs)

---

## Periodic Security Review

### Dependency Audit

- [ ] Run `cargo audit` — resolve or document all findings
- [ ] Review `deny.toml` ignore list — remove entries where patches exist
- [ ] Check for outdated dependencies: `cargo outdated`
- [ ] Review transitive dependencies for abandoned crates

### Access Review

- [ ] Audit active user accounts — deactivate unused ones
- [ ] Review admin role assignments — principle of least privilege
- [ ] Rotate JWT_SECRET (with grace period for existing tokens)
- [ ] Rotate database credentials
- [ ] Review API keys for external services

### Code Review

- [ ] Search for any new `unsafe` blocks: `grep -r "unsafe" backend/src/`
- [ ] Search for any raw SQL: `grep -r "Statement::from_string\|raw_sql" backend/src/`
- [ ] Verify no endpoints missing auth extractors
- [ ] Review audit log coverage — are all mutations logged?
- [ ] Check for hardcoded values that should be configurable

### Infrastructure

- [ ] TLS certificates not expiring soon
- [ ] Security headers still present (test with securityheaders.com)
- [ ] Backup and recovery tested
- [ ] Incident response plan reviewed

---

## Incident Response Quick Reference

### Suspected Credential Compromise

1. Rotate JWT_SECRET immediately (invalidates all sessions)
2. Force password reset for affected accounts
3. Review audit logs for unauthorized access
4. Check for data exfiltration (unusual bulk reads)
5. Notify affected users

### Suspected Data Breach

1. Identify scope (which org, which data)
2. Preserve audit logs and access logs
3. Revoke access for compromised accounts
4. Assess regulatory notification requirements (GDPR, local DR law)
5. Document timeline and remediation steps

### Dependency Vulnerability (Critical)

1. Run `cargo audit` to confirm
2. Check if vulnerable code path is exercised in your app
3. If yes: patch immediately, deploy hotfix
4. If no: add to `deny.toml` ignore with justification, schedule fix
5. Monitor for exploitation attempts
