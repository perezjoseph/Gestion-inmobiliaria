---
inclusion: fileMatch
fileMatchPattern: ["backend/src/entities/**/*", "backend/src/services/**/*", "backend/migrations/**/*", "docker-compose.yml"]
---

# Database Rules

## Technology
> Why PostgreSQL + SeaORM: PostgreSQL provides JSONB, UUID, and DECIMAL types needed for property data. SeaORM is the most mature async ORM for Rust with compile-time checked queries and SeaQL migration tooling.

- Always use PostgreSQL as the database server.
- Always use SeaORM as the async ORM.
- Never write raw SQL — all queries go through SeaORM entities.

## Connection
- Always configure database URL via `DATABASE_URL` environment variable.
- Always initialize the connection pool in `backend/src/main.rs` and pass via Actix application state.

## Migrations
- Always place migrations in `backend/migrations/`.
- Always use SeaORM migration files implementing `MigrationTrait`.
- Never modify a migration after it has been applied — always create a new migration instead.
- Always name migrations with timestamp prefix: `mYYYYMMDD_NNNNNN_description.rs`.

## Entities
- Always place SeaORM entities in `backend/src/entities/`.
- Always regenerate entities when schema changes: `sea-orm-cli generate entity`.
- Never manually edit generated entity files.

## Querying
- Always use SeaORM's query builder for all operations.
- Always use transactions (`DatabaseTransaction`) for multi-step operations.

## Data Types
- Always use UUIDs for all primary keys.
- Always use `TIMESTAMP WITH TIME ZONE` for all datetime columns.
- Always use `DECIMAL` for monetary values.
- Never use FLOAT for monetary values.
- Always use `JSONB` for flexible data (e.g., image URL arrays).
- Always use `VARCHAR` with appropriate length limits for strings.

## Indexes
- Always add indexes on all foreign key columns.
- Always add indexes on columns frequently used in WHERE clauses (email, cedula, estado, ciudad, provincia).

## Environment
- Always use Docker Compose to run PostgreSQL for local development.
- Never commit `.env` files or credentials to the repository.
