# Implementation Plan: Platform Enhancements

## Overview

Incremental implementation of 19 platform enhancements for the Gestión Inmobiliaria application. Tasks are organized to build foundational infrastructure first (database migrations, entities, shared components), then layer features on top. Each task builds on previous work, and checkpoints ensure incremental validation. All UI text is in Spanish. Backend uses Actix-web with SeaORM; frontend uses Yew + WASM with Tailwind CSS.

## Tasks

- [ ] 1. Database migrations and SeaORM entities for new tables
  - [ ] 1.1 Create migration for `registros_auditoria` table
    - Create `backend/migrations/m20250409_000001_create_registros_auditoria.rs`
    - Columns: `id` (UUID PK), `usuario_id` (UUID FK → usuarios, NOT NULL, INDEXED), `entity_type` (VARCHAR(50) NOT NULL, INDEXED), `entity_id` (UUID NOT NULL, INDEXED), `accion` (VARCHAR(20) NOT NULL), `cambios` (JSONB NOT NULL), `created_at` (TIMESTAMPTZ NOT NULL DEFAULT NOW(), INDEXED)
    - Register migration in `backend/migrations/mod.rs`
    - _Requirements: 12.4_
  - [ ] 1.2 Create migration for `documentos` table
    - Create `backend/migrations/m20250409_000002_create_documentos.rs`
    - Columns: `id` (UUID PK), `entity_type` (VARCHAR(50) NOT NULL, INDEXED), `entity_id` (UUID NOT NULL, INDEXED), `filename` (VARCHAR(255) NOT NULL), `file_path` (VARCHAR(500) NOT NULL), `mime_type` (VARCHAR(100) NOT NULL), `file_size` (BIGINT NOT NULL), `uploaded_by` (UUID FK → usuarios, NOT NULL), `created_at` (TIMESTAMPTZ NOT NULL DEFAULT NOW())
    - Register migration in `backend/migrations/mod.rs`
    - _Requirements: 14.1, 14.2, 14.3_
  - [ ] 1.3 Create migration for `configuracion` table
    - Create `backend/migrations/m20250409_000003_create_configuracion.rs`
    - Columns: `clave` (VARCHAR(100) PK), `valor` (JSONB NOT NULL), `updated_at` (TIMESTAMPTZ NOT NULL), `updated_by` (UUID FK → usuarios, NULL)
    - Seed with `clave = "tasa_cambio_dop_usd"`, `valor = {"tasa": 58.50, "actualizado": "2025-01-01"}`
    - Register migration in `backend/migrations/mod.rs`
    - _Requirements: 16.3_
  - [ ] 1.4 Create migration to add `documentos` JSONB column to `inquilinos` and `contratos`
    - Create `backend/migrations/m20250409_000004_add_documentos_columns.rs`
    - `ALTER TABLE inquilinos ADD COLUMN documentos JSONB DEFAULT '[]'::jsonb`
    - `ALTER TABLE contratos ADD COLUMN documentos JSONB DEFAULT '[]'::jsonb`
    - Register migration in `backend/migrations/mod.rs`
    - _Requirements: 14.2, 14.3_
  - [ ] 1.5 Generate SeaORM entities for new tables
    - Create `backend/src/entities/registro_auditoria.rs` with Model struct for `registros_auditoria`
    - Create `backend/src/entities/documento.rs` with Model struct for `documentos`
    - Create `backend/src/entities/configuracion.rs` with Model struct for `configuracion`
    - Update `backend/src/entities/mod.rs` to export new modules
    - Update `backend/src/entities/prelude.rs` to include new entities
    - _Requirements: 12.4, 14.1, 16.3_

- [ ] 2. Add new backend dependencies
  - Add `genpdf`, `rust_xlsxwriter`, `csv`, `calamine`, `actix-multipart`, and `actix-files` to `backend/Cargo.toml`
  - _Requirements: 2.1, 2.2, 11.1, 14.4, 19.1, 19.3_

- [ ] 3. Audit logging service
  - [ ] 3.1 Implement `backend/src/services/auditoria.rs`
    - Implement `registrar(txn: &DatabaseTransaction, entry: CreateAuditoriaEntry) -> Result<(), AppError>` to insert audit entries within the caller's transaction
    - Implement `listar(db: &DatabaseConnection, query: AuditoriaQuery) -> Result<PaginatedResponse<AuditoriaResponse>, AppError>` with filters for entity_type, entity_id, usuario_id, and date range
    - Add `AuditoriaQuery` and `AuditoriaResponse` structs to `backend/src/models/auditoria.rs`
    - Register module in `backend/src/services/mod.rs` and `backend/src/models/mod.rs`
    - _Requirements: 12.1, 12.2, 12.5_
  - [ ] 3.2 Implement `backend/src/handlers/auditoria.rs`
    - Implement `GET /api/auditoria` handler restricted to `AdminOnly` extractor
    - Register handler in `backend/src/handlers/mod.rs` and add route in `backend/src/routes.rs`
    - _Requirements: 12.2, 12.3_
  - [ ] 3.3 Integrate audit logging into existing services
    - Modify `services/propiedades.rs` create/update/delete to accept a transaction and call `auditoria::registrar` within the same transaction
    - Modify `services/inquilinos.rs` create/update/delete similarly
    - Modify `services/contratos.rs` create/update/delete similarly
    - Modify `services/pagos.rs` create/update/delete similarly
    - Update corresponding handlers to begin transactions and pass them to services
    - _Requirements: 12.1, 12.5_
  - [ ]* 3.4 Write unit tests for audit logging service
    - Test `registrar` creates correct audit entry
    - Test `listar` filters by entity_type, entity_id, usuario_id, and date range
    - Test admin-only access restriction on handler
    - _Requirements: 12.1, 12.2, 12.3_

- [ ] 4. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

