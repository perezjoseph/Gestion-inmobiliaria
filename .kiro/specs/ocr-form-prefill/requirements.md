# Requirements Document

## Introduction

Move OCR from the bulk import page into the entity creation forms where property managers actually enter data. Instead of a separate preview/confirm workflow, uploading a document photo pre-fills the form the user is already on. The user reviews the pre-filled fields naturally as part of the form, edits anything wrong, and saves with the existing submit button.

This covers four document types across four forms:
- **Pagos** — Deposit slip photos (depósitos bancarios) pre-fill the payment form.
- **Gastos** — Expense receipt photos (recibos/facturas) pre-fill the expense form.
- **Inquilinos** — Cédula photos pre-fill the tenant form (nombre, apellido, cédula).
- **Contratos** — Contract document photos pre-fill the contract form (fechas, monto mensual, depósito).

The existing OCR sidecar (`ocr-service/`), `OcrClient`, and `ocr_mapping` module are reused. The `PreviewStore` and confirm/discard endpoints become unnecessary for this flow and are left in place but unused.

## Glossary

- **OCR Pre-fill**: The process of uploading a document image, extracting fields via OCR, and populating form inputs with the extracted values — without persisting anything until the user submits the form.
- **Scan Button**: A UI button on each entity form that opens a file picker for image/PDF upload and triggers the OCR extraction flow.
- **Confidence Indicator**: A visual cue (amber border + percentage) on a pre-filled form field indicating the OCR engine's certainty about the extracted value.
- **OCR Extract Endpoint**: A backend endpoint that accepts an image, calls the OCR sidecar, maps the result to domain-specific fields, and returns them to the frontend without persisting anything.

## Requirements

### Requirement 1: Backend OCR Extract Endpoint

**User Story:** As a frontend developer, I want a single endpoint that accepts an image and returns mapped fields for a given document type, so that any form can call it and pre-fill its inputs.

#### Acceptance Criteria

1. THE Backend SHALL expose `POST /api/v1/ocr/extract` accepting a multipart image file and a `document_type` form field.
2. THE endpoint SHALL accept `document_type` values: `deposito_bancario`, `recibo_gasto`, `cedula`, `contrato`.
3. WHEN `document_type` is provided, THE endpoint SHALL skip the sidecar's automatic classification and use the provided type for field mapping.
4. WHEN `document_type` is omitted, THE endpoint SHALL use the sidecar's automatic classification.
5. THE endpoint SHALL return a JSON response with `documentType` (string), `fields` (array of objects with `name`, `value`, `label`, `confidence`), and `rawLines` (array of extracted text lines).
6. THE endpoint SHALL require `WriteAccess` (admin or gerente role).
7. THE endpoint SHALL accept JPEG, PNG, and PDF files up to 10 MB.
8. WHEN the OCR sidecar is unreachable, THE endpoint SHALL return HTTP 503 with message "Servicio OCR no disponible".
9. THE endpoint SHALL NOT persist any data — it only returns extracted fields.

### Requirement 2: Cédula Document Type Support in Sidecar

**User Story:** As a property manager, I want to scan a Dominican cédula and have the tenant's name and ID number extracted, so that I don't have to type them manually.

#### Acceptance Criteria

1. THE OCR sidecar SHALL classify documents containing keywords "CEDULA", "IDENTIDAD", "ELECTORAL", or "REPUBLICA DOMINICANA" as `document_type: "cedula"`.
2. THE OCR sidecar SHALL extract the following fields for `cedula` documents: `cedula` (formatted as NNN-NNNNNNN-N), `nombre`, `apellido`.
3. THE sidecar SHALL recognize cédula numbers both with dashes (001-1234567-8) and without (00112345678) and normalize to the dashed format.
4. THE sidecar SHALL extract nombre and apellido from the lines following the cédula header pattern, using positional heuristics matching the standard Dominican cédula layout.
5. THE `cedula` classification SHALL take priority over `deposito_bancario` and `recibo_gasto` when cédula keywords are present.

### Requirement 3: Contract Document Type Support in Sidecar

**User Story:** As a property manager, I want to scan a rental contract and have the key terms extracted, so that I can quickly create a contract record.

#### Acceptance Criteria

1. THE OCR sidecar SHALL classify documents containing keywords "CONTRATO", "ARRENDAMIENTO", or "ALQUILER" as `document_type: "contrato"`.
2. THE OCR sidecar SHALL extract the following fields for `contrato` documents: `monto_mensual`, `moneda`, `fecha_inicio`, `fecha_fin`, `deposito`.
3. THE sidecar SHALL look for monetary amounts near keywords like "CANON", "RENTA", "MENSUAL", "ALQUILER" to identify `monto_mensual`.
4. THE sidecar SHALL look for monetary amounts near keywords like "DEPOSITO", "GARANTIA" to identify `deposito`.
5. THE sidecar SHALL extract date ranges by looking for date pairs near keywords like "VIGENCIA", "PLAZO", "DESDE", "HASTA", "INICIO", "FIN".
6. THE `contrato` classification SHALL take priority over `recibo_gasto` when contract keywords are present.

### Requirement 4: Backend Field Mapping for Cédula

**User Story:** As a backend developer, I want cédula OCR results mapped to inquilino form fields, so that the frontend receives field names matching the tenant creation form.

#### Acceptance Criteria

1. THE Backend SHALL implement `map_cedula` in `ocr_mapping.rs` that converts an `OcrResult` with `document_type: "cedula"` into pre-fill fields.
2. THE mapping SHALL produce fields: `nombre` (label: "Nombre"), `apellido` (label: "Apellido"), `cedula` (label: "Cédula").
3. EACH field SHALL include the confidence score from the closest matching OCR line.
4. IF the cédula number is extracted without dashes, THE mapping SHALL normalize it to NNN-NNNNNNN-N format.

### Requirement 5: Backend Field Mapping for Contrato

**User Story:** As a backend developer, I want contract OCR results mapped to contrato form fields, so that the frontend receives field names matching the contract creation form.

#### Acceptance Criteria

1. THE Backend SHALL implement `map_contrato` in `ocr_mapping.rs` that converts an `OcrResult` with `document_type: "contrato"` into pre-fill fields.
2. THE mapping SHALL produce fields: `monto_mensual` (label: "Monto Mensual"), `moneda` (label: "Moneda"), `fecha_inicio` (label: "Fecha de Inicio"), `fecha_fin` (label: "Fecha de Fin"), `deposito` (label: "Depósito").
3. THE mapping SHALL use `parse_dr_date` for date fields and `parse_dr_currency` for monetary fields.
4. IF `monto_mensual` cannot be extracted, THE mapping SHALL return the field with an empty value and confidence 0.0.

### Requirement 6: Frontend Scan Button on Pagos Form

**User Story:** As a property manager, I want a "Escanear Recibo" button on the payment form, so that I can upload a deposit slip and have the form filled automatically.

#### Acceptance Criteria

1. THE Pagos form SHALL display a "📷 Escanear Recibo" button next to the form title when creating a new payment.
2. WHEN clicked, THE button SHALL open a file picker accepting `.jpg`, `.jpeg`, `.png`, `.pdf`.
3. WHILE the OCR request is in progress, THE button SHALL show a loading state ("Escaneando...") and be disabled.
4. WHEN the OCR response is received, THE form SHALL set: `monto` from field `monto`, `moneda` from field `moneda`, `fecha_pago` from field `fecha`, `metodo_pago` to "transferencia", `notas` from fields `referencia` and `cuenta` (concatenated).
5. THE form SHALL NOT overwrite `contrato_id` — the user must select the contract manually since that context is already established.
6. Pre-filled fields SHALL display a confidence indicator (amber border and percentage badge) for fields with confidence below 0.7.
7. THE scan button SHALL NOT appear when editing an existing payment.

### Requirement 7: Frontend Scan Button on Gastos Form

**User Story:** As a property manager, I want a "Escanear Factura" button on the expense form, so that I can upload a receipt and have the form filled automatically.

#### Acceptance Criteria

1. THE Gastos form SHALL display a "📷 Escanear Factura" button when creating a new expense.
2. WHEN the OCR response is received, THE form SHALL set: `monto` from field `monto`, `moneda` from field `moneda`, `fecha_gasto` from field `fecha`, `proveedor` from field `proveedor`, `numero_factura` from field `numero_factura`.
3. THE form SHALL NOT overwrite `propiedad_id`, `unidad_id`, or `categoria` — the user selects those manually.
4. Pre-filled fields SHALL display confidence indicators for fields below 0.7.
5. THE scan button SHALL NOT appear when editing an existing expense.

### Requirement 8: Frontend Scan Button on Inquilinos Form

**User Story:** As a property manager, I want a "📷 Escanear Cédula" button on the tenant form, so that I can scan a cédula and have the name and ID filled automatically.

#### Acceptance Criteria

1. THE Inquilinos form SHALL display a "📷 Escanear Cédula" button when creating a new tenant.
2. WHEN the OCR response is received, THE form SHALL set: `nombre` from field `nombre`, `apellido` from field `apellido`, `cedula` from field `cedula`.
3. Pre-filled fields SHALL display confidence indicators for fields below 0.7.
4. THE scan button SHALL NOT appear when editing an existing tenant.

### Requirement 9: Frontend Scan Button on Contratos Form

**User Story:** As a property manager, I want a "📷 Escanear Contrato" button on the contract form, so that I can scan a rental agreement and have the key terms filled automatically.

#### Acceptance Criteria

1. THE Contratos form SHALL display a "📷 Escanear Contrato" button when creating a new contract.
2. WHEN the OCR response is received, THE form SHALL set: `monto_mensual` from field `monto_mensual`, `moneda` from field `moneda`, `fecha_inicio` from field `fecha_inicio`, `fecha_fin` from field `fecha_fin`, `deposito` from field `deposito`.
3. THE form SHALL NOT overwrite `propiedad_id` or `inquilino_id` — the user selects those manually.
4. Pre-filled fields SHALL display confidence indicators for fields below 0.7.
5. THE scan button SHALL NOT appear when editing an existing contract.

### Requirement 10: Reusable OCR Scan Component

**User Story:** As a frontend developer, I want a reusable scan button component, so that adding OCR to any form requires minimal code.

#### Acceptance Criteria

1. THE Frontend SHALL provide a reusable `OcrScanButton` component at `frontend/src/components/common/ocr_scan_button.rs`.
2. THE component SHALL accept props: `document_type` (string), `on_result` (callback receiving field name-value-confidence tuples), `label` (button text), `disabled` (bool).
3. THE component SHALL handle the file picker, upload, loading state, and error display internally.
4. THE component SHALL call `POST /api/v1/ocr/extract` with the selected file and `document_type`.
5. WHEN an error occurs, THE component SHALL display an inline error message below the button that auto-dismisses after 5 seconds.
6. All text in the component SHALL be in Spanish.

### Requirement 11: Confidence Indicator Styling

**User Story:** As a property manager, I want to see which OCR-filled fields might be wrong, so that I know where to double-check.

#### Acceptance Criteria

1. Pre-filled fields with confidence ≥ 0.7 SHALL have no special styling (standard form appearance).
2. Pre-filled fields with confidence < 0.7 SHALL have an amber border (`#f59e0b`) and a light amber background (`#fffbeb`).
3. Pre-filled fields with confidence < 0.7 SHALL display a small badge showing the confidence percentage (e.g., "Confianza: 65%").
4. Confidence indicators SHALL be cleared when the user manually edits the field value.
5. THE Frontend SHALL provide a reusable `ConfidenceInput` wrapper component that applies these styles based on a confidence prop.

### Requirement 12: Import Page Pagos/Gastos Options

**User Story:** As a property manager, I want to import pagos and gastos from the bulk import page too, so that I can process a stack of receipts in sequence.

#### Acceptance Criteria

1. THE import page entity type dropdown SHALL include "Pagos" and "Gastos" options in addition to "Propiedades" and "Inquilinos".
2. WHEN "Pagos" is selected and an image is uploaded, THE existing OCR preview flow SHALL apply (upload → preview → confirm/discard).
3. WHEN "Gastos" is selected and an image is uploaded, THE existing OCR preview flow SHALL apply.
4. WHEN "Gastos" is selected and a CSV/XLSX is uploaded, THE existing bulk import flow SHALL apply.
