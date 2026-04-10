# OWASP Top 10 for Rust Web Apps

## A01 Broken Access Control
- Verify RBAC middleware on every endpoint
- Check that visualizador cannot write, gerente cannot manage users
- Verify resource ownership (user can only access their own data)

## A02 Cryptographic Failures
- Use argon2 for password hashing (not bcrypt, not sha256)
- JWT secrets from env vars, never hardcoded
- Use HTTPS in production

## A03 Injection
- SeaORM prevents SQL injection by default — verify no raw SQL usage
- Validate all user inputs before processing

## A05 Security Misconfiguration
- CORS must not be permissive in production
- Debug mode must be off in production
- Error responses must not leak internal details

## A07 Authentication Failures
- Rate limit login attempts
- Validate JWT on every protected request
- Short-lived tokens with refresh rotation
