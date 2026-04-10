# Auth Hardening Reference

## JWT Best Practices
- Use short-lived access tokens (15-30 min)
- Store JWT secret in environment variable, never hardcode
- Validate token expiry, issuer, and audience on every request
- Use argon2 for password hashing (already in project)
- Reject tokens with `none` algorithm

## Session Management
- Invalidate tokens on password change
- Implement token refresh rotation
- Clear tokens on logout (client-side localStorage)

## RBAC Enforcement
- Every protected endpoint must check role via middleware
- admin > gerente > visualizador hierarchy
- Verify RBAC middleware is applied in routes.rs for all /api/* routes
