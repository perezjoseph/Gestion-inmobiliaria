# Implementation Plan: Spec Gap Remediation

## Overview

Convert the feature design into a series of prompts for a code-generation LLM that will implement each step with incremental progress. Make sure that each prompt builds on the previous prompts, and ends with wiring things together. There should be no hanging or orphaned code that isn't integrated into a previous step. Focus ONLY on tasks that involve writing, modifying, or testing code.

Tasks are ordered security-first per the design's priority sequencing:

1. Cross-tenant receipt scoping (Req 1, P1) — smallest blast radius, highest severity.
2. Self-registration role + response contract (Req 2, P2).
3. Unidades `unidad_id` filter (Req 10, P6) — short, unblocks downstream tests.
4. Frontend `[INIT]` logs and Bug_Condition_PBT (Req 7) — restore diagnostics before larger UI changes.
5. Sealed contract migration, sealed PDF persistence, delete guard, and `MailClient` + `SmtpMailClient` + Mailcow wiring (Req 3, P3).
6. Document management frontend (Req 4).
7. OCR confirm persistence + tenant match + CPU-only OVMS manifest (Req 5, P4, P5).
8. WhatsApp AI multi-turn loop + `ExtractReceiptTool` + `record_extraction` (Req 8, P7).
9. Gastos completion: rentabilidad, categories, dashboard card, enum tuple fix, utility-service fields, date-range filter (Req 9, P8).
10. Platform enhancements — dashboard widgets and full PWA (Req 6). Deferred to last because it edits `frontend/index.html`, which would otherwise collide with the `[INIT]` log restoration in task 4.

All Spanish UI copy, DD/MM/YYYY dates, K8s deployment, and OVMS at `/v3` per project steering. Property tests use `crate::pbt_cases()` for iteration counts. Each PBT carries the header tag `// Feature: spec-gap-remediation, Property N`.

## Tasks

- [x] 1. Scope receipt PDF generation by `organizacion_id` (Requirement 1)
  - [x] 1.1 Update `Receipt_Service::generar_recibo` to require `organizacion_id`
    - Edit `backend/src/services/recibos.rs` to add `organizacion_id: Uuid` parameter
    - Join `pago` through `contrato` and filter `contrato::Column::OrganizacionId.eq(organizacion_id)`
    - Return `AppError::NotFound("Recibo no encontrado")` on miss
    - Emit `tracing::warn!(target: "security.cross_tenant", usuario_id, organizacion_id, pago_id, ...)` on cross-tenant miss
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 1.2 Wire `organizacion_id` from JWT Claims through the receipt handler
    - Edit `backend/src/handlers/recibos.rs` to extract `claims.organizacion_id` via `AuthenticatedUser`
    - Forward `organizacion_id` and `usuario_id` to `services::recibos::generar_recibo`
    - Leave `routes.rs` binding unchanged
    - _Requirements: 1.1, 1.2_

  - [x] 1.3 Write cross-tenant integration test for the receipt endpoint
    - Add to `backend/tests/recibos_tests.rs`: seed orgA + orgB pagos, request orgB pago as orgA user, assert `404` and empty body
    - Assert structured warning was logged via `tracing-test`
    - _Requirements: 1.3, 1.4, 1.5_

  - [x] 1.4 Write Property 1 PBT in `backend/tests/recibos_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 1: Cross-tenant receipt access never leaks`
    - Generate random `(orgA, orgB, pago)` tuples via `proptest`; iterate `crate::pbt_cases()` cases
    - Assert: every cross-tenant call yields HTTP `404` and the response body contains zero PDF bytes
    - _Requirements: 1.2, 1.3, 1.5_

- [x] 2. Self-registration role and `User` response contract (Requirement 2)
  - [x] 2.1 Persist self-registered users with `rol = "gerente"` and a fresh `Organizacion`
    - Edit `backend/src/services/auth.rs::register_new_org` to run inside a single transaction
    - Create `Organizacion` first, then `Usuario` with `rol: Set("gerente".into())` and `organizacion_id: Set(org.id)`
    - Return `User` DTO from `services::auth`; reject duplicate email with `AppError::Conflict("El correo ya está registrado")`
    - _Requirements: 2.1, 2.3, 2.4_

  - [x] 2.2 Return `201 Created` with `User` shape only from the register handler
    - Edit `backend/src/handlers/auth.rs` register handler to respond `HttpResponse::Created().json(user)`
    - Strip any token, password, or session payload from the response body
    - _Requirements: 2.2_

  - [x] 2.3 Write integration tests for register response shape and persistence
    - Add to `backend/tests/auth_tests.rs`: assert response body matches `User` schema (no `token`, no `password`)
    - Assert persisted `usuario.rol == "gerente"` and `usuario.organizacion_id` is non-null
    - Assert duplicate-email request yields `409` with `"El correo ya está registrado"`
    - _Requirements: 2.2, 2.4, 2.5_

  - [x] 2.4 Write Property 2 PBT in `backend/tests/auth_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 2: Self-registered users are always gerente`
    - Iterate `crate::pbt_cases()` random register payloads (varying any role hint)
    - Assert: persisted `rol == "gerente"`, `organizacion_id` non-null, response JSON contains exactly the `User` keys
    - _Requirements: 2.1, 2.3, 2.5_

- [x] 3. Maintenance list `unidad_id` filter (Requirement 10)
  - [x] 3.1 Add `unidad_id: Option<Uuid>` to `SolicitudListQuery`
    - Edit `backend/src/models/mantenimiento.rs` to extend the existing `SolicitudListQuery` DTO
    - _Requirements: 10.1_

  - [x] 3.2 Apply the `unidad_id` filter in the maintenance service while preserving tenant scope
    - Edit `backend/src/services/mantenimiento.rs::list` to chain `.filter(Column::OrganizacionId.eq(organizacion_id))` first, then `.apply_if(q.unidad_id, |sel, uid| sel.filter(Column::UnidadId.eq(uid)))`
    - A `unidad_id` from another organizacion yields an empty list (no existence leak)
    - _Requirements: 10.2, 10.3, 10.4_

  - [x] 3.3 Write integration tests in `backend/tests/mantenimiento_tests.rs`
    - Seed solicitudes across two organizations and two unidades; assert filter returns only matching rows scoped to caller's `organizacion_id`
    - Assert `unidad_id` from another organizacion returns `[]`
    - _Requirements: 10.2, 10.4, 10.5_

  - [x] 3.4 Write Property 6 PBT in `backend/tests/mantenimiento_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 6: Maintenance filter respects unidad_id and tenant scope`
    - Iterate `crate::pbt_cases()` random datasets and `(org, unidad_id)` query inputs
    - Assert: every returned row satisfies `row.organizacion_id == caller_org` AND (`unidad_id.is_none()` OR `row.unidad_id == unidad_id`)
    - _Requirements: 10.2, 10.3, 10.4_

- [ ] 4. Restore Yew bootstrap `[INIT]` logs and write the Bug_Condition_PBT (Requirement 7)
  - [x] 4.1 Emit pre-renderer marker in `frontend/src/main.rs`
    - `web_sys::console::log_1(&"[INIT] pre-renderer".into());` before `yew::Renderer::<App>::new().render()`
    - Keep the log in production builds; do not gate behind `cfg(debug_assertions)`
    - _Requirements: 7.1, 7.4_

  - [x] 4.2 Emit app-mount, route-resolution, and switch markers in `frontend/src/app.rs`
    - Inside `App` function component: `[INIT] app mounted`
    - Inside the `Switch<Route>` render closure: `[INIT] route resolution`
    - Inside the `switch(route)` matcher: `[INIT] switch`
    - _Requirements: 7.1, 7.4_

  - [x] 4.3 Emit auth-check and first-route-rendered markers in `frontend/src/components/common/protected_route.rs`
    - Inside `ProtectedRoute`: `[INIT] auth check`
    - Inside the auth-success branch: `[INIT] first route rendered`
    - _Requirements: 7.1, 7.4_

  - [x] 4.4 Restore `frontend/tests/init_logging_tests.rs` Bug_Condition_PBT
    - `// Feature: spec-gap-remediation, Bug_Condition_PBT: All [INIT] markers present at boot`
    - Use `wasm-bindgen-test` headless to boot the app under random `(route, auth_state)` permutations
    - Iterate `crate::pbt_cases()` cases; assert every marker (`pre-renderer`, `app mounted`, `route resolution`, `switch`, `auth check`, `first route rendered`) appears in console output
    - On failure, the counterexample SHALL identify the missing stage
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [x] 5. Checkpoint — verify P1, P2, Req 7, Req 10 land cleanly
  - Run `cargo test --workspace` and `trunk build --release` (or equivalent frontend test runner) to confirm the four highest-priority remediations pass before touching shared subsystems.
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Sealed signed-contract document and Mailcow SMTP wiring (Requirement 3)

  > **Deviation note**: Requirement 3.3 specifies HTTP `409 Conflict` for sealed-document delete attempts. The design overrides this with HTTP `403 Forbidden` (`AppError::Forbidden("No se puede eliminar un documento sellado")`). Per user guidance the design wins. Implementation in 6.3 and tests in 6.9 MUST assert `403`, not `409`. See the Note section at the bottom of this file.

  - [x] 6.1 Add the `documento_origen_id` column migration
    - Create `backend/src/migrations/m20260415_001_add_documento_origen_id.rs`
    - Add nullable `documento_origen_id UUID` to `documentos` with FK `fk_documento_origen_contrato → contratos.id ON DELETE SET NULL`
    - Re-export from `backend/src/migrations/mod.rs` and append to the migrator vector
    - _Requirements: 3.1_

  - [x] 6.2 Persist the sealed PDF as a `Documento` after signing completes
    - Edit `backend/src/services/firmas.rs` to add `generar_pdf_sellado(db, contrato, organizacion_id)`
    - Render PDF via existing `render_contrato_pdf`, write to `uploads/contratos/{contrato_id}/sellado.pdf`
    - Insert `documento::ActiveModel` with `entity_type = "contrato"`, `entity_id = contrato.id`, `documento_origen_id = Some(contrato.id)`, `sellado = true`, `organizacion_id`
    - _Requirements: 3.2_

  - [x] 6.3 Reject sealed-document deletion with HTTP `403 Forbidden`
    - Edit `backend/src/services/documentos.rs::eliminar` to load with org scope, then return `AppError::Forbidden("No se puede eliminar un documento sellado")` when `doc.sellado || doc.documento_origen_id.is_some()`
    - Verify `AppError::Forbidden` maps to HTTP `403` in the central error layer; add the variant if missing
    - **Implementation MUST return 403 per design, not 409 from the requirements text** (see deviation note above)
    - _Requirements: 3.3_

  - [x] 6.4 Define the `MailClient` trait, `OutgoingMail`, and `SmtpConfig`
    - Create `backend/src/services/mail/mod.rs`, `client.rs`, `message.rs`
    - `client.rs`: `#[async_trait] pub trait MailClient: Send + Sync { async fn send(&self, msg: OutgoingMail) -> Result<(), AppError>; }` plus `pub struct OutgoingMail { to, subject, body_html, body_text }`
    - `message.rs`: Spanish builder `signature_link_mail(contrato, link) -> OutgoingMail`
    - Add `SmtpConfig::from_env()` to `backend/src/config.rs` reading `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`
    - _Requirements: 3.4_

  - [x] 6.5 Implement `SmtpMailClient` via `lettre`
    - Create `backend/src/services/mail/smtp.rs` with `pub struct SmtpMailClient { transport: AsyncSmtpTransport<Tokio1Executor>, from: Mailbox }`
    - `from_config(cfg)` builds `starttls_relay(host).port(port).credentials(creds).build()`
    - `MailClient::send` builds a multipart-alternative message and maps SMTP errors to `AppError::BadGateway("No se pudo enviar el correo")` while logging without echoing credentials
    - _Requirements: 3.4, 3.6_

  - [x] 6.6 Wire `MailClient` into `AppState` and the signing flow
    - Edit `backend/src/app.rs` to construct `Arc<dyn MailClient + Send + Sync>` from `SmtpConfig::from_env()` and store it on `AppState`
    - Edit `backend/src/services/firmas.rs::enviar_email_firma(mail: &dyn MailClient, inquilino, contrato, link)` to call `mail.send(signature_link_mail(...))` with Spanish subject/body
    - Wire the call site in the signing handler to use the trait object from `AppState`
    - _Requirements: 3.5, 3.6_

  - [x] 6.7 Add `mailcow-smtp` envFrom to backend deployment
    - Edit `infra/k8s/app/backend.yaml` to add `envFrom: [{ secretRef: { name: mailcow-smtp } }]`
    - Document the expected secret keys (`SMTP_HOST=mail.myhomeva.us`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM=no-reply@myhomeva.us`) in a manifest comment
    - _Requirements: 3.4_

  - [x] 6.8 Write Property 3 PBT and integration tests in `backend/tests/firmas_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 3: Sealed-document deletion is rejected`
    - Iterate `crate::pbt_cases()` random sealed-document datasets; assert delete returns HTTP `403` (see deviation note), the row remains in DB, and the file on disk is unchanged
    - _Requirements: 3.2, 3.3_

  - [x] 6.9 Write mail integration test using `lettre::AsyncFileTransport`
    - Add to `backend/tests/firmas_tests.rs`: swap `MailClient` for a file-transport implementation, trigger the signing flow, parse the persisted `.eml`, assert Spanish subject (`"Firma electrónica de su contrato #..."`), Spanish body, recipient address, and presence of the signing link
    - Add a separate gated test (`#[ignore]`) under `backend/tests/firmas_tests.rs` for real Mailcow staging
    - _Requirements: 3.5, 3.6_

- [x] 7. Document management frontend (Requirement 4)
  - [x] 7.1 Activate the `Verification_Action` button in `frontend/src/components/feature/verification_badge.rs`
    - Wire the button click to `api::put::<DocumentoStatus, ()>("/documentos/{id}/verificar", &DocumentoStatus { status })` via `spawn_local`
    - Update the displayed status from local state without a full page reload
    - Hide the button when `current_user().rol == "visualizador"`
    - _Requirements: 4.1, 4.2, 4.6_

  - [x] 7.2 Render the `Compliance_Profile` per-property view via `frontend/src/components/feature/compliance_badge.rs`
    - Render the response of `GET /documentos/cumplimiento/{entity_type}/{entity_id}`
    - Show required, present, missing, and expiring-within-30-days lists in Spanish
    - Mount on `frontend/src/pages/inquilinos.rs`, `frontend/src/pages/propiedades.rs`, and `frontend/src/pages/contratos.rs` detail views
    - _Requirements: 4.3, 4.7_

  - [x] 7.3 Create `Expiring_Docs_List` page and route
    - New page `frontend/src/pages/documentos_por_vencer.rs` calling `GET /documentos/por-vencer`
    - Sort ascending by `fecha_expiracion`; render dates as DD/MM/YYYY
    - Add `#[at("/documentos/por-vencer")] DocumentosPorVencer` to the `Route` enum and switch
    - _Requirements: 4.4, 4.7_

  - [x] 7.4 Render `Compliance_Counters` on the dashboard
    - Extend `frontend/src/types/dashboard.rs::DashboardStats` with `documentos_vencidos`, `documentos_por_vencer`, `entidades_incompletas`
    - Add three counter cards in `frontend/src/pages/dashboard.rs` scoped to caller's `organizacion_id`
    - All labels in Spanish (`"Documentos vencidos"`, `"Por vencer"`, `"Entidades incompletas"`)
    - _Requirements: 4.5, 4.7_

  - [x] 7.5 Write component tests for visualizador hiding and Spanish copy
    - Use `wasm-bindgen-test` to assert the verification button is absent when `rol == "visualizador"` and present for `admin`/`gerente`
    - Assert all rendered labels and statuses are in Spanish
    - _Requirements: 4.1, 4.6, 4.7_

- [x] 8. Checkpoint — confirm sealed-doc + mail + document-management land cleanly
  - Run `cargo test --workspace` and the frontend test runner.
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. OCR confirm persistence, tenant match, and CPU-only OVMS (Requirement 5)
  - [x] 9.1 Implement synchronous `Confirmar_Preview` insert in `backend/src/services/chatbot.rs`
    - Add `confirmar_preview(db, preview, organizacion_id, usuario_id)` running inside `db.begin()` transaction
    - Idempotency: lookup by `preview.id` within org first; return existing `ConfirmedEntity` if present
    - `Document_Type::Recibo` → `services::pagos::crear`; `Document_Type::Gasto` → `services::gastos::crear`
    - Record the preview→entity mapping in `preview_index` before commit
    - _Requirements: 5.1, 5.2, 5.3_

  - [x] 9.2 Add best-effort tenant matcher `services::ocr_mapping::map_deposito`
    - Create `backend/src/services/ocr_mapping.rs`
    - Filter `inquilino` by `organizacion_id` and a `LIKE %trimmed%` predicate over `concat(nombre, ' ', apellido)`
    - Return `Some(id)` only when candidate set has exactly one match; otherwise `None` (never wrong)
    - Wire into `confirmar_preview` for the `Recibo` branch; persist with `inquilino_id = NULL` when `None`
    - _Requirements: 5.4, 5.5_

  - [x] 9.3 Map OCR validation failures to HTTP `422`
    - In `confirmar_preview`, fail with `AppError::UnprocessableEntity("Datos de OCR inválidos")` when extraction values fail validation; do not insert any row
    - _Requirements: 5.7_

  - [x] 9.4 Pin OVMS to CPU-only in the K8s manifest
    - Edit `infra/k8s/app/ovms.yaml`: remove the `gpu.intel.com/i915` resource request and any reference to the `i915` device-plugin DaemonSet
    - Set `env: [{ name: OPENVINO_DEVICE, value: "CPU" }, { name: TARGET_DEVICE, value: "CPU" }]`
    - _Requirements: 5.6_

  - [x] 9.5 Write Property 4 PBT in `backend/tests/importacion_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 4: OCR confirm inserts exactly one row, idempotently`
    - Iterate `crate::pbt_cases()` random valid `OcrPreview` payloads; assert single-row insertion for `recibo`/`gasto`, and that two confirms with the same `preview_id` produce one row
    - For invalid extraction payloads assert HTTP `422` and unchanged row counts
    - _Requirements: 5.1, 5.2, 5.3, 5.7_

  - [x] 9.6 Write Property 5 PBT in `backend/tests/importacion_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 5: Tenant match is best-effort and never wrong`
    - Iterate `crate::pbt_cases()` random `(name, inquilino_dataset, organizacion_id)` triples
    - Assert: returned `Some(id)` only when candidate set within `organizacion_id` has size 1; never returns an inquilino from another org; otherwise `None`
    - _Requirements: 5.4, 5.5_

- [x] 10. WhatsApp AI multi-turn agent loop and `Extract_Receipt_Tool` wiring (Requirement 8)
  - [x] 10.1 Implement `Multi_Turn_Agent_Loop` in `backend/src/services/ai_module.rs::invoke_agent`
    - Loop up to `TURN_LIMIT = 5` turns, calling `agent.completion(&chat_history)`
    - On `AgentResponse::Final(text)` return `AgentOutcome::Final { text, history }`
    - On `AgentResponse::ToolCalls(calls)` execute each tool, push tool results back into `chat_history`, and continue
    - On exhaustion return `AgentOutcome::TurnLimitReached { text: "Disculpa, no pude completar tu solicitud. Inténtalo de nuevo, por favor.".into(), history }` and log the conversation id
    - On tool error, surface the error to the agent as a tool result so it can recover within the same conversation
    - All inference targets `https://ovms.<ns>.svc.cluster.local/v3` — never an external provider
    - _Requirements: 8.1, 8.4, 8.5, 8.6_

  - [x] 10.2 Register `Extract_Receipt_Tool` with the Rig agent
    - Create `backend/src/services/ai_module/tools/extract_receipt.rs` implementing `rig::Tool` with `NAME = "extract_receipt"`
    - `call(args)`: fetch media via `media_store.fetch(args.media_id)`, then `ocr.extract(&bytes)` (which targets OVMS `/v3`), returning `PaymentReceipt`
    - Register the tool when constructing the agent in `services::ai_module`
    - _Requirements: 8.2, 8.6_

  - [x] 10.3 Wire `record_extraction` into the post-loop path
    - Edit `backend/src/services/chatbot.rs`: when `invoke_agent` returns `Final` and the history contains a successful `ExtractReceiptTool` result, call `record_extraction(db, receipt, organizacion_id, usuario_id)` which delegates to `confirmar_preview`
    - _Requirements: 8.3_

  - [x] 10.4 Write Property 7 PBT in `backend/tests/ai_module_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 7: Multi-turn agent loop terminates`
    - Iterate `crate::pbt_cases()` random tool-call sequences from a stub LLM (final, tool, error, looping, etc.)
    - Assert: `invoke_agent` always returns within 5 turns with either `Final` text or the Spanish `TurnLimitReached` fallback; never loops indefinitely
    - _Requirements: 8.1, 8.4_

  - [x] 10.5 Write integration test for `record_extraction` post-loop wiring
    - Add to `backend/tests/chatbot_pbt.rs`: stub a successful `ExtractReceiptTool` result, run `invoke_agent`, assert a `Pago` row is persisted via `confirmar_preview`
    - _Requirements: 8.3, 8.5_

- [ ] 11. Gastos completion (Requirement 9)
  - [x] 11.1 Add `Utility_Service_Fields` to the `Gasto` entity, DTOs, and migration
    - Add a migration `m20260415_002_add_gasto_utility_fields.rs` with nullable `proveedor TEXT`, `numero_cuenta TEXT`, `periodo_inicio DATE`, `periodo_fin DATE` columns
    - Regenerate `backend/src/entities/gasto.rs` (or hand-add columns to match)
    - Extend `backend/src/models/gasto.rs::CreateGasto` and `UpdateGasto` with the four optional fields
    - _Requirements: 9.5_

  - [x] 11.2 Validate the `categoria` enum and reject out-of-set values with HTTP `422`
    - Edit `backend/src/services/gastos.rs::crear` and `actualizar` to validate against `{ mantenimiento, servicios, impuestos, seguro, administracion, otros }`
    - Return `AppError::UnprocessableEntity("Categoría de gasto no válida")` on miss
    - _Requirements: 9.4_

  - [x] 11.3 Apply the `Date_Range_Filter` on the gastos list endpoint
    - Edit `backend/src/services/gastos.rs::list` to accept `fecha_desde`, `fecha_hasta` from the filter DTO
    - Return `AppError::BadRequest("fecha_desde no puede ser posterior a fecha_hasta")` when `fecha_desde > fecha_hasta`
    - Apply `apply_if(filter.fecha_desde, |q, d| q.filter(Column::FechaGasto.gte(d)))` and analogous `lte` for `fecha_hasta`, after the org-scope filter
    - _Requirements: 9.6, 9.7_

  - [x] 11.4 Fix the `gastos.rs:48` enum-tuple typo on the frontend
    - Edit `frontend/src/pages/gastos.rs` line 48: change to `("servicio_publico", "Servicio Público")`
    - _Requirements: 9.4_

  - [x] 11.5 Add the `Rentabilidad_View` tab to the reportes page
    - Edit `frontend/src/pages/reportes.rs` to add a `Rentabilidad` tab calling `/reportes/rentabilidad?fecha_desde=…&fecha_hasta=…`
    - Render a per-`Propiedad` table of income (`sum(Pago WHERE estado = 'pagado')`) minus expenses (`sum(Gasto WHERE estado = 'pagado')`)
    - Wire `Descargar PDF` and `Descargar Excel` buttons to existing export endpoints; all labels in Spanish
    - _Requirements: 9.1, 9.8_

  - [x] 11.6 Create the `Category_Summary` page
    - New `frontend/src/pages/categorias_gastos.rs` calling `GET /gastos/resumen-categorias`
    - Render a sortable table of `categoría / total / count`; Spanish labels; DD/MM/YYYY where dates are shown
    - Add `#[at("/categorias-gastos")] CategoriasGastos` to the `Route` enum
    - _Requirements: 9.2, 9.8_

  - [x] 11.7 Add the `Expense_Card` to the dashboard
    - Edit `frontend/src/pages/dashboard.rs` to render a card showing total pending expenses, total paid expenses for the current month, and overdue count
    - Backed by `GET /dashboard/gastos-comparacion`
    - _Requirements: 9.3, 9.8_

  - [x] 11.8 Render utility fields conditionally and format currency/dates
    - Edit `frontend/src/components/feature/gasto_form.rs` to render `proveedor`, `numero_cuenta`, `periodo_inicio`, `periodo_fin` only when `categoria == "servicios"`
    - Display all amounts with `moneda` symbol and two decimals; render dates as DD/MM/YYYY
    - _Requirements: 9.5, 9.8_

  - [x] 11.9 Add `fecha_desde`/`fecha_hasta` date pickers to the gastos filter bar
    - Edit `frontend/src/components/feature/gasto_filter_bar.rs` to add the two date inputs and propagate state via callback
    - _Requirements: 9.6, 9.8_

  - [x] 11.10 Write Property 8 PBT in `backend/tests/gastos_pbt.rs`
    - `// Feature: spec-gap-remediation, Property 8: Date-range filter on gastos is sound and complete`
    - Iterate `crate::pbt_cases()` random gastos datasets and `(fecha_desde, fecha_hasta)` tuples
    - Assert: every returned row satisfies `fecha_desde <= row.fecha_gasto <= fecha_hasta` and belongs to caller's `organizacion_id`; `fecha_desde > fecha_hasta` yields HTTP `400`
    - _Requirements: 9.6, 9.7_

  - [x] 11.11 Write `categoria` enum and utility-fields tests in `backend/tests/gastos_tests.rs`
    - Property-style negative test (`crate::pbt_cases()` random non-enum strings) asserting `422` with Spanish message
    - Round-trip test for `proveedor`, `numero_cuenta`, `periodo_inicio`, `periodo_fin` create→read
    - _Requirements: 9.4, 9.5_

- [x] 12. Checkpoint — confirm OCR, WhatsApp AI, and gastos land cleanly
  - Run `cargo test --workspace` and the frontend test runner.
  - Ensure all tests pass, ask the user if questions arise.

- [x] 13. Platform enhancements — dashboard widgets and full PWA (Requirement 6)

  > Deferred to last because tasks 13.5 and 13.6 edit `frontend/index.html`, which would otherwise collide with the `[INIT]` log restoration in task 4. Run task 4 to completion (and the bug-condition PBT in 4.4) before starting this group.

  - [x] 13.1 Render `Contratos_Por_Vencer_Widget` on the dashboard
    - Edit `frontend/src/pages/dashboard.rs` to render three buckets (30/60/90 days) sorted ascending by `fecha_fin`
    - Backed by the existing list endpoint with a `dias` query param
    - Spanish labels (`"Contratos por vencer"`)
    - _Requirements: 6.1, 6.9_

  - [x] 13.2 Render `Occupancy_Chart` on the dashboard
    - Wrap `/dashboard/ocupacion-tendencia`; render a 12-month line chart via the existing chart component
    - Spanish title (`"Ocupación últimos 12 meses"`)
    - _Requirements: 6.2, 6.9_

  - [x] 13.3 Render `Upcoming_Payments_Widget` on the dashboard
    - Call `/pagos?estado=pendiente&hasta=YYYY-MM-DD` (next 30 days), sorted ascending by `fecha_vencimiento`
    - Spanish labels; DD/MM/YYYY date format
    - _Requirements: 6.3, 6.9_

  - [x] 13.4 Create the `Calendar_View` page
    - New `frontend/src/pages/calendario.rs` overlaying contratos (start/end), pagos (due dates), and mantenimientos (appointments) for the user's `organizacion_id`
    - Add `#[at("/calendario")] Calendario` to the `Route` enum
    - Spanish entries; split into sub-components if `html!` exceeds 150 lines
    - _Requirements: 6.4, 6.9_

  - [x] 13.5 Ship the `PWA_Manifest`
    - Create `frontend/manifest.webmanifest` declaring `name`, `short_name`, icons (192, 512), `theme_color`, `background_color`, `display: "standalone"`
    - _Requirements: 6.5_

  - [x] 13.6 Register the `Service_Worker`
    - Create `frontend/service-worker.js` precaching `/index.html`, `/main.wasm`, `/main.js`, theme CSS; add runtime cache and offline fallback
    - Edit `frontend/index.html` to add `<link rel="manifest">` and the SW registration script (do not disturb the `[INIT]` log markers added in task 4)
    - _Requirements: 6.6_

  - [x] 13.7 Implement `IndexedDB_Cache` wrapper
    - Create `frontend/src/services/idb_cache.rs` wrapping the `idb` crate with `read_list<T>(store, key) -> Option<Vec<T>>` and `write_list<T>(store, key, value)` via `serde_wasm_bindgen`
    - _Requirements: 6.7_

  - [x] 13.8 Implement the `Online_Hook` and online listener
    - Create `frontend/src/services/online.rs` subscribing to browser `online`/`offline` events via `gloo-events`
    - Create `frontend/src/hooks/use_online.rs` exposing `pub fn use_online() -> bool`
    - _Requirements: 6.8_

  - [x] 13.9 Add the `offline_guard` component
    - Create `frontend/src/components/common/offline_guard.rs` wrapping any submit button and rendering it disabled when `use_online()` is false
    - Wire it into create/update/delete buttons across propiedades, inquilinos, contratos, pagos, gastos forms
    - _Requirements: 6.8_

  - [x] 13.10 Apply cache-first reads in list pages
    - Edit `frontend/src/pages/{propiedades,inquilinos,contratos,pagos,gastos}.rs` to: try `idb_cache::read_list` first when offline; on successful network read, write-through via `idb_cache::write_list`
    - _Requirements: 6.7, 6.8_

  - [x] 13.11 Write tests for manifest, service-worker registration, and `use_online`
    - `wasm-bindgen-test` cases asserting `manifest.webmanifest` ships with the required fields, the SW registers, `use_online` reflects toggled events, and `offline_guard` disables buttons when offline
    - _Requirements: 6.5, 6.6, 6.8, 6.9_

- [x] 14. Final checkpoint — full workspace verification
  - Run `cargo test --workspace` and the frontend test runner one last time
  - Verify K8s manifests parse (`kubectl --dry-run=client apply -f infra/k8s/app/`) and that the OVMS endpoint is `/v3` with `OPENVINO_DEVICE=CPU`
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional test-only sub-tasks and can be skipped for a faster MVP. Top-level tasks are never optional.
- Each task references granular sub-requirement numbers (e.g. `9.6`, not just `9`) for traceability.
- Property tests use `crate::pbt_cases()` for iteration counts and live in `backend/tests/{domain}_pbt.rs` per the testing steering. Each PBT carries the `// Feature: spec-gap-remediation, Property N: <text>` header.
- Checkpoints (tasks 5, 8, 12, 14) run `cargo test --workspace` and the frontend test runner; they are not implementation tasks and are not included in the dependency graph below.
- All user-facing copy is Spanish; dates render DD/MM/YYYY; currency renders with `moneda` symbol and two decimals.
- Inference always targets OVMS at `/v3`; never an external provider. Deployment is K8s-only; ignore `docker-compose.dev.yml`.
- **Deviation flag — sealed-document delete status code**: Requirement 3.3 specifies HTTP `409 Conflict`, but the design specifies HTTP `403 Forbidden` (`AppError::Forbidden("No se puede eliminar un documento sellado")`). Per user guidance the design takes precedence. Task 6.3 implements `403`; Task 6.8's PBT and any related integration tests assert `403`, not `409`. If a future spec revision wants to align the requirements text with the implementation, update Requirement 3.3 to `403`.

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "2.1", "3.1", "4.1", "6.1", "6.4", "9.4", "11.1", "11.4"] },
    { "id": 1, "tasks": ["1.2", "2.2", "3.2", "4.2", "6.5", "6.6", "11.2", "11.3"] },
    { "id": 2, "tasks": ["4.3", "6.2", "6.3", "6.7", "7.1", "7.2", "7.3", "7.4", "9.1", "9.2", "9.3", "10.1", "10.2", "11.5", "11.6", "11.7", "11.8", "11.9"] },
    { "id": 3, "tasks": ["1.3", "1.4", "2.3", "2.4", "3.3", "3.4", "4.4", "6.8", "6.9", "7.5", "9.5", "9.6", "10.3", "11.10", "11.11"] },
    { "id": 4, "tasks": ["10.4", "10.5", "13.1", "13.2", "13.3", "13.4", "13.7", "13.8"] },
    { "id": 5, "tasks": ["13.5", "13.6", "13.9", "13.10"] },
    { "id": 6, "tasks": ["13.11"] }
  ]
}
```
