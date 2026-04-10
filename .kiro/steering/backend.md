---
inclusion: fileMatch
fileMatchPattern: ["backend/**/*.rs", "backend/Cargo.toml"]
---

# Backend Rules

## Framework
> Why Actix-web: Mature async Rust framework with strong middleware support, proven production performance, and the largest ecosystem of plugins for web apps.

- Always use Actix-web as the web framework.
- Always make handlers async using `async/await`.
- Always register routes in `backend/src/routes.rs`.

## Architecture
- Always follow layered architecture: handlers -> services -> entities.
- Always place handlers in `backend/src/handlers/` — parse requests, validate input, call services, return HTTP responses.
- Always place services in `backend/src/services/` — business logic only, no HTTP concerns.
- Always place SeaORM entities in `backend/src/entities/` — never manually edit generated entities.
- Always place request/response DTOs in `backend/src/models/`.

## Serialization
- Always use `serde` + `serde_json` for all serialization.
- Always use `#[serde(rename_all = "camelCase")]` for API response fields.
- Always accept both camelCase and snake_case in request bodies.

## Authentication
- Always use `jsonwebtoken` crate for JWT issuance and validation.
- Always use `argon2` for password hashing.
- Never store plaintext passwords.
- Always place JWT middleware in `backend/src/middleware/auth.rs`.
- Always place RBAC middleware in `backend/src/middleware/rbac.rs`.
- Always extract authenticated user from request extensions, not from body.

## Error Responses
- Always return errors as JSON: `{ "error": "error_type", "message": "Description" }`.
- Always use proper HTTP status codes: 400, 401, 403, 404, 409, 422, 500.

## Configuration
- Always use environment variables for configuration (database URL, JWT secret, server port).
- Never hardcode secrets or credentials.

## Logging
- Always use `tracing` for structured logging, never `println!` or `log`.
- Always wrap the Actix-web app with `tracing-actix-web::TracingLogger`.
- Always initialize `tracing-subscriber` with `EnvFilter` in `main.rs`.

## Dependencies
- Always check `backend/Cargo.toml` before assuming a crate is available.
- Always prefer crates already in the dependency list.
- Always justify and explicitly add new dependencies to Cargo.toml.
