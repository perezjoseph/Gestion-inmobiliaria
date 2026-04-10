# Input Validation Reference

## Validation Patterns for Actix-web
- Validate all request body fields before passing to services
- Use custom validators for Dominican Republic formats (cedula: 11 digits, phone: 10 digits)
- Validate date ranges (fecha_inicio < fecha_fin)
- Validate monetary amounts (> 0, reasonable upper bounds)
- Validate enum values (estado, tipo_propiedad, moneda, rol)
- Sanitize string inputs (trim whitespace, check length limits)

## SeaORM Query Safety
- Always use parameterized queries via SeaORM query builder
- Never interpolate user input into raw SQL
- Validate IDs before using in queries
- Use transactions for multi-step operations
