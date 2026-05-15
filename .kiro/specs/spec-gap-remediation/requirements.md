# Requirements Document

## Introduction

This omnibus specification consolidates ten remediation items spanning critical security defects, contract integrity gaps, document management features, OCR persistence, dashboard and PWA enhancements, frontend diagnostics, WhatsApp AI completion, expense workflows, and unidad filtering. The platform is a Dominican Republic property management system with multi-tenant isolation by `organizacion_id` carried in JWT Claims, Spanish UI text, DOP/USD currency handling, DD/MM/YYYY date display, and self-hosted deployment on Kubernetes (k3s). The backend is Rust 2024 (Actix-web + SeaORM), the frontend is Yew + Trunk, and AI inference uses OVMS at endpoint `/v3` with OCR exposed as a Rig tool that the LLM invokes. Items are ordered by severity: cross-tenant data leakage and authorization gaps first, followed by integrity, then features and polish.

## Glossary

- **Receipt_Service**: Backend service `services::recibos::generar_recibo` responsible for generating payment receipt PDFs.
- **Mail_Client**: Generic Rust trait abstracting outbound email delivery; consumed by services that need to send mail.
- **SmtpMailClient**: Concrete `Mail_Client` implementation backed by the `lettre` crate, configured for Mailcow SMTP.
- **Mailcow_SMTP**: Self-hosted Mailcow mail server reachable at host `mail.myhomeva.us`, sending from `no-reply@myhomeva.us`, with credentials stored in K8s Secret `mailcow-smtp`.
- **Sealed_Document**: A finalized signed contract PDF persisted as a `Documento` record and protected from deletion.
- **Documento_Origen_Id**: Column on the `documentos` table that records the originating contract identifier for sealed signed-contract PDFs.
- **Tenant_Match**: Best-effort lookup that links an OCR-extracted receipt to an existing `inquilino` by name; absence of a match does not block insertion.
- **Confirmar_Preview**: WhatsApp AI confirmation handler that finalizes an OCR preview into a persisted record (receipt or expense) based on `document_type`.
- **Document_Type**: Discriminator on a confirmed OCR preview indicating either `recibo` or `gasto`.
- **Multi_Turn_Agent_Loop**: Rig agent loop in the WhatsApp AI service that supports tool calls across multiple turns until a final assistant message is produced.
- **Extract_Receipt_Tool**: Rig tool exposed to the WhatsApp AI agent that performs OCR extraction on an image and returns structured receipt fields.
- **Record_Extraction**: Function that persists an OCR extraction result into the appropriate domain entity.
- **Compliance_Profile**: Per-property document compliance summary aggregating required, present, and expiring documents.
- **Verification_Action**: Admin/gerente UI control that updates a `Documento` verification status (verified, rejected, pending).
- **Expiring_Docs_List**: Frontend page listing documents whose expiration date falls within a configured horizon.
- **Compliance_Counters**: Dashboard metrics summarizing total required, verified, expired, and expiring documents across the organization.
- **Online_Hook**: Yew hook (`use_online`) exposing browser online/offline status to components.
- **Service_Worker**: Browser service worker registered by the frontend to provide offline cache and PWA install support.
- **PWA_Manifest**: `manifest.webmanifest` file describing the installable Yew app (name, icons, theme, display mode).
- **IndexedDB_Cache**: Browser IndexedDB store used by list pages for cache-first reads when offline.
- **Init_Logs**: `[INIT]` console log markers emitted by the Yew frontend bootstrap path used to diagnose loading failures.
- **Bug_Condition_PBT**: Property-based test that exercises the frontend bootstrap path and asserts the bug condition (missing `[INIT]` logs) cannot reoccur.
- **Solicitud_List_Query**: Backend query DTO used to filter `SolicitudMantenimiento` listings.
- **Rentabilidad_View**: Frontend page summarizing income minus expenses (profitability) per property.
- **Category_Summary**: Aggregate of `Gasto` totals grouped by `categoria`.
- **Expense_Card**: Dashboard widget summarizing recent and pending expenses.
- **Utility_Service_Fields**: `Gasto` fields capturing utility-service metadata (e.g., `proveedor`, `numero_cuenta`, `periodo_inicio`, `periodo_fin`).
- **Date_Range_Filter**: Query parameters `fecha_desde` and `fecha_hasta` for filtering expenses by date.
- **Contratos_Por_Vencer_Widget**: Dashboard widget listing contracts approaching expiration.
- **Occupancy_Chart**: Dashboard chart showing the ratio of occupied versus available units over time.
- **Upcoming_Payments_Widget**: Dashboard widget listing payments with `fecha_vencimiento` in the next 30 days.
- **Calendar_View**: Frontend calendar showing contract milestones, payment due dates, and maintenance appointments.
- **Organizacion_Id**: Tenant identifier carried in JWT Claims and used to scope every multi-tenant query.
- **OVMS**: OpenVINO Model Server hosted in-cluster, reachable at endpoint `/v3`, configured with `OPENVINO_DEVICE=CPU`.

## Requirements

### Requirement 1: Cross-Tenant Receipt PDF Leak Remediation

**User Story:** As a platform operator, I want receipt PDF generation to be scoped strictly by `organizacion_id`, so that no user can retrieve a receipt belonging to another tenant.

#### Acceptance Criteria

1. WHEN a request reaches `Receipt_Service`, THE Receipt_Service SHALL require an `organizacion_id` parameter sourced from the authenticated user's JWT Claims.
2. WHEN `Receipt_Service` loads the `Pago` for the requested receipt, THE Receipt_Service SHALL join through `Contrato` and `Propiedad` and filter by the caller's `organizacion_id`.
3. IF the receipt's resolved `organizacion_id` does not equal the caller's `organizacion_id`, THEN THE Receipt_Service SHALL return a `404 Not Found` error and SHALL NOT include the receipt PDF in the response body.
4. THE Receipt_Service SHALL log a structured warning containing the caller's `usuario_id`, `organizacion_id`, and the requested `pago_id` whenever a cross-tenant access attempt is detected.
5. THE Receipt_Service SHALL be covered by an integration test that asserts a user from organization A receives `404` when requesting a receipt that belongs to organization B.

### Requirement 2: Landlord Self-Registration Role and Response Contract

**User Story:** As a landlord registering for the platform, I want to be assigned the `gerente` role automatically and receive a response containing only my user record, so that the registration contract is consistent and I can manage my own properties immediately.

#### Acceptance Criteria

1. WHEN a self-registration request is submitted to the public register endpoint, THE Backend SHALL create the `Usuario` with `rol` set to `gerente`.
2. WHEN self-registration succeeds, THE Backend SHALL return an HTTP `201 Created` response whose body matches the `User` DTO schema and SHALL NOT include a token, password, or session payload.
3. WHEN self-registration succeeds, THE Backend SHALL create or attach an `Organizacion` record so that the new `Usuario` has a non-null `organizacion_id`.
4. IF the submitted email already exists, THEN THE Backend SHALL return HTTP `409 Conflict` with a Spanish error message and SHALL NOT create a duplicate `Usuario`.
5. THE register endpoint SHALL be covered by tests asserting the response body equals the `User` DTO and that `rol = "gerente"` is persisted.

### Requirement 3: Contract Signing Sealed Document and Tenant Notification

**User Story:** As a property manager, I want signed contracts to produce a sealed PDF that cannot be deleted and to trigger an email notification to the tenant via Mailcow SMTP, so that signed contracts are auditable and tenants receive their signing link.

#### Acceptance Criteria

1. THE `documentos` table SHALL include a nullable column `documento_origen_id` of type UUID referencing the originating `contrato.id`.
2. WHEN a contract signing flow completes, THE Backend SHALL persist the sealed PDF as a `Documento` record with `entity_type = "contrato"`, `entity_id = contrato.id`, and `documento_origen_id = contrato.id`.
3. IF a delete request targets a `Documento` whose `documento_origen_id` is not null, THEN THE Backend SHALL return HTTP `409 Conflict` with a Spanish error message indicating the document is sealed.
4. THE Backend SHALL define a `Mail_Client` trait and a `SmtpMailClient` implementation backed by `lettre`, configured from K8s Secret `mailcow-smtp` with host `mail.myhomeva.us` and from address `no-reply@myhomeva.us`.
5. WHEN a tenant signing link is generated, THE Backend SHALL invoke `Mail_Client::send` to deliver the link to the tenant's email address with Spanish subject and body.
6. IF `Mail_Client::send` returns an error, THEN THE Backend SHALL log the error with `contrato_id` and `inquilino_id` and SHALL return HTTP `502 Bad Gateway` to the caller without exposing SMTP credentials.

### Requirement 4: Document Management Frontend

**User Story:** As an admin or gerente, I want frontend pages to verify documents, view per-property compliance, see expiring documents, and read compliance counters from the dashboard, so that I can manage document compliance end-to-end.

#### Acceptance Criteria

1. THE Frontend SHALL provide a `Verification_Action` control on the document detail view that allows users with `admin` or `gerente` role to set verification status to `verificado`, `rechazado`, or `pendiente`.
2. WHEN a `Verification_Action` is submitted, THE Frontend SHALL call the existing verification endpoint and SHALL update the displayed status without a full page reload.
3. THE Frontend SHALL provide a `Compliance_Profile` view per `Propiedad` that lists required document types, indicates which are present, which are missing, and which are expiring within 30 days.
4. THE Frontend SHALL provide an `Expiring_Docs_List` page showing documents whose `fecha_expiracion` falls within the next 30 days, sorted ascending by expiration date.
5. THE Frontend SHALL render `Compliance_Counters` on the dashboard showing totals for required, verified, expired, and expiring documents scoped to the user's `organizacion_id`.
6. WHERE a user has the `visualizador` role, THE Frontend SHALL hide the `Verification_Action` control while still rendering the read-only views.
7. THE Frontend SHALL display all labels, statuses, and messages in Spanish and SHALL format dates as DD/MM/YYYY.

### Requirement 5: OCR Confirm Persistence with Tenant Match and CPU-Only OCR

**User Story:** As a tenant or property manager using WhatsApp AI, I want a confirmed OCR preview to be persisted synchronously into the correct domain entity and matched to a tenant where possible, while OCR runs on CPU only, so that confirmations produce reliable records on the existing hardware.

#### Acceptance Criteria

1. WHEN `Confirmar_Preview` is invoked, THE Backend SHALL perform a synchronous database insert before returning the confirmation response.
2. WHERE `Document_Type` equals `recibo`, THE Backend SHALL insert a `Pago` row using the OCR extraction values.
3. WHERE `Document_Type` equals `gasto`, THE Backend SHALL insert a `Gasto` row using the OCR extraction values.
4. WHEN persisting a `recibo` confirmation, THE Backend SHALL attempt a best-effort `Tenant_Match` against `Inquilino` by full name within the caller's `organizacion_id`.
5. IF `Tenant_Match` finds no candidate, THEN THE Backend SHALL persist the row with `inquilino_id = NULL` and SHALL NOT return an error.
6. THE OCR Kubernetes manifest SHALL set `OPENVINO_DEVICE=CPU` and SHALL NOT include the GPU device plugin.
7. IF the OCR extraction payload fails validation, THEN THE Backend SHALL return HTTP `422 Unprocessable Entity` with a Spanish error message and SHALL NOT insert any row.

### Requirement 6: Platform Enhancements — Dashboard Widgets and Full PWA

**User Story:** As a property manager, I want the dashboard to show contracts nearing expiration, occupancy, upcoming payments, and a calendar, and I want the frontend to install as a full PWA with offline list reads, so that I can monitor the portfolio at a glance and use the app offline.

#### Acceptance Criteria

1. THE Dashboard SHALL render a `Contratos_Por_Vencer_Widget` listing contracts whose `fecha_fin` falls within the next 60 days, sorted ascending by `fecha_fin`.
2. THE Dashboard SHALL render an `Occupancy_Chart` displaying the ratio of `Unidad` records with `estado = "ocupada"` to total units over the last 12 months.
3. THE Dashboard SHALL render an `Upcoming_Payments_Widget` listing pagos whose `fecha_vencimiento` falls within the next 30 days and whose `estado` is `pendiente`.
4. THE Frontend SHALL provide a `Calendar_View` page that displays contract start/end dates, payment due dates, and maintenance appointments for the user's `organizacion_id`.
5. THE Frontend SHALL ship a `PWA_Manifest` (`manifest.webmanifest`) declaring name, short name, icons, theme color, background color, and `display = "standalone"`.
6. THE Frontend SHALL register a `Service_Worker` that precaches the application shell and serves cached responses when the network is unavailable.
7. WHILE the browser is offline, THE Frontend SHALL serve list pages (propiedades, inquilinos, contratos, pagos, gastos) from `IndexedDB_Cache` using a cache-first strategy.
8. THE Frontend SHALL expose an `Online_Hook` (`use_online`) and SHALL disable create, update, and delete buttons in components when the hook reports offline.
9. THE Frontend SHALL display all dashboard labels, widget headers, and calendar entries in Spanish.

### Requirement 7: Frontend Loading Diagnostics and Bug-Condition PBT

**User Story:** As a developer diagnosing frontend loading failures, I want `[INIT]` console logs restored at each bootstrap stage and a property-based test that asserts those logs appear, so that loading regressions surface early and visibly.

#### Acceptance Criteria

1. WHEN the Yew frontend boots, THE Frontend SHALL emit `[INIT]` console log markers at each of: WASM module loaded, app mounted, router initialized, and first route rendered.
2. THE Frontend SHALL include a `Bug_Condition_PBT` that loads the bootstrap path under headless conditions and asserts all four `[INIT]` markers appear in console output.
3. IF any `[INIT]` marker is missing during the `Bug_Condition_PBT`, THEN the test SHALL fail with a counterexample identifying the missing stage.
4. THE Frontend SHALL keep the `[INIT]` log statements in production builds and SHALL NOT gate them behind a debug flag.

### Requirement 8: WhatsApp AI Multi-Turn Agent Loop and Receipt Tool Wiring

**User Story:** As a tenant interacting via WhatsApp, I want the AI agent to handle multi-turn conversations including tool calls for receipt extraction and to persist extractions through the existing `record_extraction` path, so that I can submit receipts conversationally and have them stored correctly.

#### Acceptance Criteria

1. THE WhatsApp AI service SHALL implement a `Multi_Turn_Agent_Loop` that issues tool calls and consumes tool results until the model returns a final assistant message or a configured turn limit is reached.
2. THE WhatsApp AI service SHALL register `Extract_Receipt_Tool` with the Rig agent so that the LLM may invoke it on user-supplied images.
3. WHEN `Extract_Receipt_Tool` returns a successful extraction, THE WhatsApp AI service SHALL invoke `Record_Extraction` to persist the result into the appropriate domain entity.
4. IF `Multi_Turn_Agent_Loop` reaches the turn limit without a final assistant message, THEN THE WhatsApp AI service SHALL return a Spanish fallback message to the user and SHALL log the conversation id.
5. WHEN any tool invocation returns an error, THE WhatsApp AI service SHALL surface the error to the agent as a tool result so that the model can recover or apologize within the same conversation.
6. THE WhatsApp AI service SHALL route every model call to `OVMS` at endpoint `/v3` and SHALL NOT call any external inference provider.

### Requirement 9: Gastos Completion — Rentabilidad, Categories, Dashboard, Utility Fields, Filters

**User Story:** As a gerente managing expenses, I want a profitability view, a category summary, a dashboard expense card, a corrected category enum, utility-service fields on `Gasto`, and a date-range filter, so that expense tracking is complete and aligned with how I operate.

#### Acceptance Criteria

1. THE Frontend SHALL provide a `Rentabilidad_View` showing income (sum of `Pago` with `estado = "pagado"`) minus expenses (sum of `Gasto` with `estado = "pagado"`) per `Propiedad` for a selectable date range.
2. THE Frontend SHALL provide a `Category_Summary` view aggregating `Gasto` totals grouped by `categoria` for the user's `organizacion_id`.
3. THE Dashboard SHALL render an `Expense_Card` showing total pending expenses, total paid expenses for the current month, and a count of overdue expenses.
4. THE Backend SHALL define the `Gasto.categoria` enum to include `mantenimiento`, `servicios`, `impuestos`, `seguro`, `administracion`, and `otros` and SHALL reject values outside this set with HTTP `422`.
5. THE `Gasto` entity SHALL include `Utility_Service_Fields`: `proveedor`, `numero_cuenta`, `periodo_inicio`, and `periodo_fin`, all nullable.
6. THE Backend SHALL accept a `Date_Range_Filter` (`fecha_desde`, `fecha_hasta`) on the `Gasto` list endpoint and SHALL return only gastos whose `fecha_gasto` falls within the inclusive range.
7. IF `fecha_desde` is greater than `fecha_hasta`, THEN THE Backend SHALL return HTTP `400 Bad Request` with a Spanish error message.
8. THE Frontend SHALL render all gastos amounts with the corresponding `moneda` symbol and two decimal places, and SHALL render dates as DD/MM/YYYY.

### Requirement 10: Unidades — Maintenance List Filter

**User Story:** As a property manager, I want to filter maintenance requests by `unidad_id`, so that I can focus on a single unit's tickets.

#### Acceptance Criteria

1. THE `Solicitud_List_Query` DTO SHALL include an optional `unidad_id` field of type UUID.
2. WHEN `Solicitud_List_Query.unidad_id` is provided, THE Backend SHALL return only `SolicitudMantenimiento` rows whose `unidad_id` equals the supplied value and whose `organizacion_id` equals the caller's `organizacion_id`.
3. WHEN `Solicitud_List_Query.unidad_id` is omitted, THE Backend SHALL return results unfiltered by `unidad_id`.
4. IF the supplied `unidad_id` does not belong to the caller's `organizacion_id`, THEN THE Backend SHALL return an empty list and SHALL NOT leak existence information about the unit.
5. THE maintenance list endpoint SHALL be covered by a test asserting that filtering by `unidad_id` returns only rows for that unit and respects tenant scoping.
