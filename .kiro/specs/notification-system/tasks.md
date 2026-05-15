# Implementation Plan: Sistema de Notificaciones

## Overview

Implementar el sistema completo de notificaciones siguiendo la arquitectura existente (handlers → services → entities). Se crea una tabla nueva `notificaciones`, su entidad SeaORM, modelos request/response, servicio con generador de notificaciones y funciones CRUD, handlers con auth extractors, rutas, integración con mantenimiento, componente de campana en navbar, y página de listado frontend. Se sigue el patrón exacto de los módulos existentes como referencia.

## Tasks

- [x] 1. Database migration
  - [x] 1.1 Create migration `m20250501_000001_create_notificaciones`
    - Create file `backend/migrations/m20250501_000001_create_notificaciones.rs`
    - Define table `notificaciones` with all columns per design: id (UUID PK), tipo (VARCHAR(50) NOT NULL), titulo (VARCHAR(500) NOT NULL), mensaje (TEXT NOT NULL), leida (BOOLEAN NOT NULL DEFAULT false), entity_type (VARCHAR(50) NOT NULL), entity_id (UUID NOT NULL), usuario_id (UUID NOT NULL FK → usuarios(id)), organizacion_id (UUID NOT NULL FK → organizaciones(id)), created_at (TIMESTAMPTZ NOT NULL DEFAULT now())
    - Add indexes: `idx_notificaciones_usuario_id`, `idx_notificaciones_usuario_leida` on (usuario_id, leida), `idx_notificaciones_tipo_entity` UNIQUE on (tipo, entity_type, entity_id, usuario_id), `idx_notificaciones_organizacion_id`, `idx_notificaciones_created_at`
    - Follow the exact pattern from `m20250408_000005_create_pagos.rs` for table creation, FK constraints, and index creation
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 1.2 Register migration in `backend/migrations/mod.rs`
    - Add `pub mod` declaration for the new migration module
    - Add migration to the `Migrator::migrations()` vec
    - _Requirements: 1.1_

- [x] 2. SeaORM entity
  - [x] 2.1 Create entity `backend/src/entities/notificacion.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "notificaciones")]`
    - All fields per design: id (Uuid PK), tipo (String), titulo (String), mensaje (String with column_type Text), leida (bool), entity_type (String), entity_id (Uuid), usuario_id (Uuid), organizacion_id (Uuid), created_at (DateTimeWithTimeZone)
    - Define `Relation` enum with `belongs_to Usuario` and `belongs_to Organizacion`
    - Implement `Related` traits for Usuario and Organizacion
    - Follow the pattern from `backend/src/entities/pago.rs`
    - _Requirements: 1.1, 1.4_

  - [x] 2.2 Register entity in `backend/src/entities/mod.rs`
    - Add `pub mod notificacion;`
    - _Requirements: 1.1_

- [x] 3. Request/response models
  - [x] 3.1 Extend `backend/src/models/notificacion.rs`
    - Keep existing `PagoVencido` struct and its tests unchanged
    - Add `NotificacionResponse` (Serialize, camelCase): id, tipo, titulo, mensaje, leida, entity_type, entity_id, usuario_id, created_at
    - Add `NotificacionListQuery` (Deserialize, camelCase): leida (Option<bool>), tipo (Option<String>), page (Option<u64>), per_page (Option<u64>)
    - Add `ConteoNoLeidasResponse` (Serialize, camelCase): count (u64)
    - Add `MarcarTodasResponse` (Serialize, camelCase): actualizadas (u64)
    - Add `GenerarNotificacionesResponse` (Serialize, camelCase): pago_vencido (u64), contrato_por_vencer (u64), documento_vencido (u64), total (u64)
    - _Requirements: 2.1, 3.1, 5.2, 10.2_

  - [x] 3.2 Write unit tests for new model serialization/deserialization
    - Add tests to existing `#[cfg(test)]` module in `backend/src/models/notificacion.rs`
    - Test `NotificacionResponse` serialization produces camelCase
    - Test `NotificacionListQuery` deserialization with optional fields
    - Test `ConteoNoLeidasResponse`, `MarcarTodasResponse`, `GenerarNotificacionesResponse` serialization
    - _Requirements: 2.1, 3.1_

- [x] 4. Service layer
  - [x] 4.1 Refactor `backend/src/services/notificaciones.rs`
    - Keep existing `listar_pagos_vencidos` function unchanged
    - Add constants: `TIPOS_NOTIFICACION` (`pago_vencido`, `contrato_por_vencer`, `documento_vencido`, `mantenimiento_actualizado`), `DIAS_ANTICIPACION` (30)
    - Implement `From<notificacion::Model>` for `NotificacionResponse`
    - Implement `listar(db, usuario_id, query)`: paginated list filtered by usuario_id, optional filters for leida and tipo, ordered by created_at DESC
    - Implement `conteo_no_leidas(db, usuario_id)`: count where usuario_id matches and leida == false
    - Implement `marcar_leida(db, id, usuario_id)`: find by id AND usuario_id (returns NotFound if either doesn't match), update leida to true if not already, return NotificacionResponse
    - Implement `marcar_todas_leidas(db, usuario_id)`: update_many setting leida=true where usuario_id matches and leida==false, return rows_affected
    - Implement helper `usuarios_activos_organizacion(db, organizacion_id)`: query usuarios where organizacion_id matches and activo==true, return Vec<Uuid>
    - Implement helper `existe_notificacion(db, tipo, entity_type, entity_id, usuario_id)`: check if notification with same combination exists, return bool
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3_

  - [x] 4.2 Implement notification generators in `backend/src/services/notificaciones.rs`
    - Implement `generar_pagos_vencidos(db, organizacion_id)`: query pagos with estado="pendiente" and fecha_vencimiento < today, join with contratos and propiedades for titulo, get active users, create notifications with deduplication check, return count
    - Implement `generar_contratos_por_vencer(db, organizacion_id)`: query contratos with estado="activo" and fecha_fin between today and today+30, join with propiedades for titulo, get active users, create notifications with deduplication check, return count
    - Implement `generar_documentos_vencidos(db, organizacion_id)`: query documentos with fecha_vencimiento not null and fecha_vencimiento <= today+30, get active users of the organization, create notifications with deduplication check, return count
    - Implement `generar_notificaciones(db, organizacion_id)`: orchestrate all three generators, return GenerarNotificacionesResponse with counts per type and total
    - Implement `crear_notificacion_mantenimiento(db, solicitud_id, titulo_solicitud, estado_anterior, estado_nuevo, organizacion_id)`: create mantenimiento_actualizado notifications for all active users of the organization, return count. No deduplication (each state change is unique).
    - Use batch queries with `is_in()` and HashMap lookups to avoid N+1 queries
    - _Requirements: 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 10.1, 10.2_

  - [x] 4.3 Write unit tests for service functions
    - Add `#[cfg(test)]` module in `backend/src/services/notificaciones.rs`
    - Test `From<Model>` conversion for `NotificacionResponse`
    - Test tipo validation (valid and invalid values)
    - _Requirements: 1.2, 4.1_

- [x] 5. Integrate with mantenimiento service
  - [x] 5.1 Modify `backend/src/services/mantenimiento.rs` — `cambiar_estado` function
    - After successful state transition and before audit registration, call `notificaciones::crear_notificacion_mantenimiento` with best-effort approach (log warning on failure, don't revert state change)
    - Pass solicitud_id, titulo, old estado, new estado, and organizacion_id
    - Add `use crate::services::notificaciones;` import
    - Need to retrieve organizacion_id from the solicitud — the solicitud_mantenimiento entity has organizacion_id field
    - _Requirements: 9.1, 9.2, 9.3_

- [x] 6. Handlers
  - [x] 6.1 Refactor `backend/src/handlers/notificaciones.rs`
    - Keep existing `pagos_vencidos` handler unchanged
    - Add `listar(db, claims, query: Query<NotificacionListQuery>)` → Ok(json)
    - Add `conteo_no_leidas(db, claims)` → Ok(json)
    - Add `marcar_leida(db, claims, path: Path<Uuid>)` → Ok(json)
    - Add `marcar_todas_leidas(db, claims)` → Ok(json)
    - Add `generar(db, access: WriteAccess)` → Ok(json), uses claims.organizacion_id
    - Follow the exact pattern from `backend/src/handlers/pagos.rs`
    - _Requirements: 2.1, 3.1, 4.1, 5.1, 10.1, 10.3_

- [x] 7. Route registration
  - [x] 7.1 Update notificaciones routes in `backend/src/routes.rs`
    - Replace the existing single-route `/notificaciones` scope with the expanded scope
    - Register routes in order: static paths first (/pagos-vencidos, /no-leidas/conteo, /leer-todas, /generar), then dynamic (/{id}/leer), then root ("")
    - Preserve the existing /pagos-vencidos route
    - _Requirements: 2.1, 3.1, 4.1, 5.1, 10.1_

- [x] 8. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo check --workspace` and `cargo test --workspace` to verify all new modules compile and existing tests still pass.

- [x] 9. Frontend types
  - [x] 9.1 Extend `frontend/src/types/notificacion.rs`
    - Keep existing `PagoVencido` struct unchanged
    - Add `Notificacion` struct (Deserialize, Serialize, Clone, PartialEq, camelCase): id (String), tipo (String), titulo (String), mensaje (String), leida (bool), entity_type (String), entity_id (String), usuario_id (String), created_at (String)
    - Add `ConteoNoLeidas` struct (Deserialize, Serialize, Clone, PartialEq, camelCase): count (u64)
    - Add `MarcarTodasResponse` struct (Deserialize, Serialize, Clone, PartialEq, camelCase): actualizadas (u64)
    - Add `GenerarNotificacionesResponse` struct (Deserialize, Serialize, Clone, PartialEq, camelCase): pago_vencido (u64), contrato_por_vencer (u64), documento_vencido (u64), total (u64)
    - _Requirements: 11.1, 12.1_

- [x] 10. Frontend notification bell component
  - [x] 10.1 Create `frontend/src/components/layout/notification_bell.rs`
    - Implement `NotificationBell` functional component
    - On mount (`use_effect_with((), ...)`), call `api_get::<ConteoNoLeidas>("/notificaciones/no-leidas/conteo")` to fetch unread count
    - Render SVG bell icon
    - If count > 0, render a badge span with the count number positioned over the bell
    - If count == 0, render bell without badge
    - On click, navigate to `Route::Notificaciones` using `use_navigator()`
    - Keep component under 100 lines to avoid WASM OOM
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

  - [x] 10.2 Integrate NotificationBell into Navbar
    - Add `mod notification_bell;` and import in `frontend/src/components/layout/navbar.rs` or `frontend/src/components/layout/mod.rs`
    - Place `<NotificationBell />` in the Navbar component between `<NavbarSearch />` and `<ThemeToggle />`
    - _Requirements: 11.1_

- [x] 11. Frontend notificaciones page
  - [x] 11.1 Create `frontend/src/pages/notificaciones.rs`
    - Implement `Notificaciones` functional component with paginated list view
    - Fetch notifications via `api_get::<PaginatedResponse<Notificacion>>("/notificaciones?page=...")`
    - Display table with columns: Tipo (icon/badge), Título, Mensaje, Estado (leída/no leída), Fecha (DD/MM/YYYY)
    - Differentiate unread notifications visually (bold text or highlighted background)
    - Add "Marcar todas como leídas" button calling `api_put::<MarcarTodasResponse, _>("/notificaciones/leer-todas", &())`
    - Add per-row "Marcar como leída" button calling `api_put::<Notificacion, _>("/notificaciones/{id}/leer", &())`
    - Add filters: dropdown for tipo (pago_vencido, contrato_por_vencer, documento_vencido, mantenimiento_actualizado), dropdown for leida (todas, leídas, no leídas)
    - Type indicators: pago_vencido=red, contrato_por_vencer=orange, documento_vencido=yellow, mantenimiento_actualizado=blue
    - All text in Spanish, dates in DD/MM/YYYY format
    - Split into sub-components if html! blocks exceed 150 lines
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8_

  - [x] 11.2 Register page module in `frontend/src/pages/mod.rs`
    - Add `pub mod notificaciones;`
    - _Requirements: 12.1_

  - [x] 11.3 Add route and navigation
    - Add `Notificaciones` variant to `Route` enum in `frontend/src/app.rs` with `#[at("/notificaciones")]`
    - Add match arm in `switch()` function: `Route::Notificaciones => html! { <ProtectedRoute><Notificaciones /></ProtectedRoute> }`
    - Add import for `crate::pages::notificaciones::Notificaciones`
    - _Requirements: 12.1_

- [x] 12. Checkpoint — Ensure full project compiles
  - Run `cargo check --workspace` to verify frontend and backend compile together.

- [x] 13. Integration tests
  - [x] 13.1 Create `backend/tests/notificaciones_tests.rs`
    - Test listing empty notifications → paginated empty response
    - Test generating notifications → verify counts per type
    - Test listing after generation → notifications appear
    - Test filtering by tipo → only matching type returned
    - Test filtering by leida → only matching state returned
    - Test unread count → correct number
    - Test mark one as read → leida=true, count decremented
    - Test mark non-existent → 404
    - Test mark another user's notification → 404
    - Test mark all as read → count returned, subsequent count = 0
    - Test generate as visualizador → 403
    - Test generate twice → second returns zero new (deduplication)
    - Test mantenimiento state change generates mantenimiento_actualizado notification
    - Test legacy /pagos-vencidos endpoint still works
    - _Requirements: 2.1–2.4, 3.1–3.2, 4.1–4.4, 5.1–5.3, 6.1–6.3, 7.1–7.3, 8.1–8.3, 9.1–9.3, 10.1–10.3_

- [x] 14. Property-based tests
  - [x] 14.1 Write property test: Listing returns only user's notifications (P1)
    - **Property 1: Listing returns only user's notifications**
    - **Validates: Requirements 2.4, 3.2**
    - Generate notifications for multiple random user IDs, list for one user, verify all returned records have matching usuario_id

  - [x] 14.2 Write property test: List ordering invariant (P2)
    - **Property 2: List ordering invariant**
    - **Validates: Requirements 2.1**
    - Create multiple notifications, list without filters, verify created_at descending order for every consecutive pair

  - [x] 14.3 Write property test: Filtering returns only matching records (P3)
    - **Property 3: Filtering returns only matching records**
    - **Validates: Requirements 2.2, 2.3**
    - Create notifications with varied tipos and leida states, filter by each, verify all returned records match the filter

  - [x] 14.4 Write property test: Unread count consistency (P4)
    - **Property 4: Unread count consistency**
    - **Validates: Requirements 3.1, 4.1, 5.1, 5.3**
    - Generate random mix of read/unread notifications, verify count matches, mark one as read, verify count-1, mark all, verify 0

  - [x] 14.5 Write property test: Mark as read is idempotent (P5)
    - **Property 5: Mark as read is idempotent**
    - **Validates: Requirements 4.4**
    - Mark notification as read twice, verify result and count are identical both times

  - [x] 14.6 Write property test: Mark all updates only unread (P6)
    - **Property 6: Mark all as read updates only unread**
    - **Validates: Requirements 5.1, 5.2, 5.3**
    - Generate mix of read/unread, mark all, verify returned count equals previously unread count

  - [x] 14.7 Write property test: Notification deduplication (P7)
    - **Property 7: Notification deduplication**
    - **Validates: Requirements 6.3, 7.3, 8.3**
    - Generate notifications, invoke generator twice, verify zero new on second invocation

  - [x] 14.8 Write property test: Generated notifications have correct fields (P8)
    - **Property 8: Generated notifications have correct fields**
    - **Validates: Requirements 1.2, 6.2, 7.2, 8.2**
    - Generate notifications, verify tipo is valid, entity_type matches tipo, titulo non-empty, mensaje non-empty

  - [x] 14.9 Write property test: Cross-user isolation (P9)
    - **Property 9: Cross-user isolation on mark operations**
    - **Validates: Requirements 4.3**
    - Generate notifications for two users, mark as read for user A, verify user B's notifications and count unchanged

  - [x] 14.10 Write property test: New notifications default to unread (P10)
    - **Property 10: New notifications default to unread**
    - **Validates: Requirements 1.3**
    - Generate notifications, verify all have leida == false

- [x] 15. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass.

## Notes

- The existing endpoint GET /api/v1/notificaciones/pagos-vencidos is preserved for backward compatibility
- The notification generator is a callable function, not a background scheduler
- Mantenimiento notifications are created inline during state changes, not via the batch generator
- Deduplication uses a UNIQUE index on (tipo, entity_type, entity_id, usuario_id) for the batch generator types
- Mantenimiento notifications skip deduplication since each state change is a distinct event
- All UI text is in Spanish
- Property tests validate universal correctness properties from the design document
