# Implementation Plan: OCR Form Pre-fill

## Overview

Move OCR extraction into entity creation forms via a stateless `POST /api/v1/ocr/extract` endpoint. Add cédula and contrato document types to the sidecar, create backend field mappings, build reusable `OcrScanButton` and `ConfidenceInput` frontend components, and integrate scanning into the Pagos, Gastos, Inquilinos, and Contratos forms. Also add Pagos/Gastos options to the import page dropdown.

## Tasks

- [x] 1. Add backend types and extract endpoint
  - [x] 1.1 Add `ExtractResponse` and `ExtractField` structs to `backend/src/models/ocr.rs`
    - Add `ExtractResponse { document_type, fields, raw_lines }` and `ExtractField { name, value, label, confidence }` with `#[serde(rename_all = "camelCase")]`
    - Keep existing `ImportPreview`, `PreviewField`, `OcrResult`, `OcrLine` types unchanged
    - _Requirements: 1.5_

  - [x] 1.2 Implement `ocr_extract` handler in `backend/src/handlers/ocr.rs`
    - Create new `handlers/ocr.rs` module, re-export in `handlers/mod.rs`
    - Accept `Multipart` with `file` (required, JPEG/PNG/PDF, ≤10MB) and `document_type` (optional string)
    - Require `WriteAccess` extractor for authorization
    - Validate file type and size, return 422 on invalid input
    - Call `OcrClient::extract()`, pass `document_type` to sidecar when provided
    - Match on `document_type` to call the appropriate `map_*` function, return raw fields for unknown types
    - Return `ExtractResponse` JSON
    - On sidecar connection error, return 503 with "Servicio OCR no disponible"
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 1.7, 1.8, 1.9_

  - [x] 1.3 Register the `/api/v1/ocr/extract` route in `backend/src/routes.rs`
    - Add `web::scope("/ocr").route("/extract", web::post().to(handlers::ocr::ocr_extract))` to the `/api/v1` scope
    - _Requirements: 1.1_

  - [x] 1.4 Update `OcrClient` to pass optional `document_type` to the sidecar
    - Modify `OcrClient::extract()` in `backend/src/services/ocr_client.rs` to accept an optional `document_type` parameter and forward it as a form field to the sidecar's `/ocr/extract` endpoint
    - _Requirements: 1.3, 1.4_

- [x] 2. Add cédula and contrato mapping functions
  - [x] 2.1 Implement `normalize_cedula` in `backend/src/services/ocr_mapping.rs`
    - Strip non-digit characters, format as `NNN-NNNNNNN-N` if exactly 11 digits, return cleaned string otherwise
    - _Requirements: 2.3, 4.4_

  - [x] 2.2 Write property test for cédula normalization (Property 1)
    - **Property 1: Cédula normalization is idempotent and format-preserving**
    - Generate random 11-digit strings with/without dashes; verify output is `NNN-NNNNNNN-N` and applying `normalize_cedula` again yields the same result
    - Add to `backend/src/services/ocr_mapping_pbt.rs`, add `proptest` to `[dev-dependencies]` in `backend/Cargo.toml`
    - **Validates: Requirements 2.3, 4.4**

  - [x] 2.3 Implement `map_cedula` in `backend/src/services/ocr_mapping.rs`
    - Convert `OcrResult` with `document_type: "cedula"` into `Vec<ExtractField>` with fields: `cedula` (label: "Cédula"), `nombre` (label: "Nombre"), `apellido` (label: "Apellido")
    - Use `normalize_cedula` on the cédula value
    - Use `field_confidence` to set confidence from matching OCR lines
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [x] 2.4 Write property test for map_cedula output (Property 6)
    - **Property 6: map_cedula produces exactly the required fields**
    - Generate random `OcrResult` with cedula/nombre/apellido structured_fields; verify exactly 3 fields with correct names/labels and cédula in `NNN-NNNNNNN-N` format
    - **Validates: Requirements 4.2, 4.4**

  - [x] 2.5 Implement `map_contrato` in `backend/src/services/ocr_mapping.rs`
    - Convert `OcrResult` with `document_type: "contrato"` into `Vec<ExtractField>` with fields: `monto_mensual` (label: "Monto Mensual"), `moneda` (label: "Moneda"), `fecha_inicio` (label: "Fecha de Inicio"), `fecha_fin` (label: "Fecha de Fin"), `deposito` (label: "Depósito")
    - Use `parse_dr_date` for dates, `parse_dr_currency` for monetary fields
    - If `monto_mensual` is missing, return field with empty value and confidence 0.0
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [x] 2.6 Write property test for map_contrato output (Property 7)
    - **Property 7: map_contrato produces the required fields with graceful degradation**
    - Generate random `OcrResult` with/without contrato fields; verify correct field names/labels and graceful degradation when `monto_mensual` is absent
    - **Validates: Requirements 5.2, 5.4**

  - [x] 2.7 Implement `map_deposito_extract` and `map_gasto_extract` in `backend/src/services/ocr_mapping.rs`
    - Reuse existing `map_deposito`/`map_gasto` logic but return `Vec<ExtractField>` instead of `ImportPreview`
    - _Requirements: 1.5_

  - [x] 2.8 Write property test for confidence propagation (Property 8)
    - **Property 8: Field confidence matches highest matching OCR line**
    - Generate random `OcrResult` with known line confidences; verify each mapped field's confidence equals the max confidence among matching lines
    - **Validates: Requirements 4.3**

  - [x] 2.9 Write unit tests for mapping functions
    - Test `normalize_cedula` edge cases: empty string, wrong length, already formatted, no digits
    - Test `map_cedula` with full fields, missing fields, malformed cédula
    - Test `map_contrato` with full fields, missing monto_mensual, unparseable dates
    - Test `map_deposito_extract` and `map_gasto_extract` produce correct `ExtractField` vectors
    - Add to `backend/src/services/ocr_mapping_tests.rs`
    - _Requirements: 4.2, 4.4, 5.2, 5.4_

- [x] 3. Checkpoint — Backend mapping layer
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Add cédula and contrato support to OCR sidecar
  - [x] 4.1 Add cédula and contrato classification to `_classify_document` in `ocr-service/main.py`
    - Add `cedula` classification (keywords: CEDULA, IDENTIDAD, ELECTORAL, REPUBLICA DOMINICANA) with highest priority
    - Add `contrato` classification (keywords: CONTRATO, ARRENDAMIENTO, ALQUILER) with priority above `recibo_gasto`
    - Reorder checks: cedula → contrato → deposito_bancario → recibo_gasto → unknown
    - _Requirements: 2.1, 2.5, 3.1, 3.6_

  - [x] 4.2 Write property test for cédula classification priority (Property 2)
    - **Property 2: Cédula classification takes priority**
    - Generate random text containing at least one cédula keyword plus other document keywords; verify classification returns "cedula"
    - Add to `ocr-service/test_classification_pbt.py` using `hypothesis`
    - **Validates: Requirements 2.1, 2.5**

  - [x] 4.3 Write property test for contrato classification priority (Property 3)
    - **Property 3: Contract classification takes priority over expense**
    - Generate random text containing at least one contract keyword plus expense keywords; verify classification returns "contrato"
    - **Validates: Requirements 3.1, 3.6**

  - [x] 4.4 Add cédula field extraction to `_extract_structured_fields` in `ocr-service/main.py`
    - Extract `cedula` via regex `\d{3}-?\d{7}-?\d`, normalize to `NNN-NNNNNNN-N`
    - Extract `nombre` and `apellido` using positional heuristics (lines after header)
    - _Requirements: 2.2, 2.3, 2.4_

  - [x] 4.5 Add contrato field extraction to `_extract_structured_fields` in `ocr-service/main.py`
    - Extract `monto_mensual` from monetary amounts near CANON/RENTA/MENSUAL/ALQUILER keywords
    - Extract `moneda` from RD$/US$ prefix detection
    - Extract `fecha_inicio` from dates near DESDE/INICIO/VIGENCIA keywords
    - Extract `fecha_fin` from dates near HASTA/FIN/VENCIMIENTO keywords
    - Extract `deposito` from monetary amounts near DEPOSITO/GARANTIA keywords
    - _Requirements: 3.2, 3.3, 3.4, 3.5_

  - [x] 4.6 Add optional `document_type` parameter to sidecar `/ocr/extract` endpoint
    - Accept `document_type` as an optional form field in the FastAPI endpoint
    - When provided, skip `_classify_document` and use the provided type directly
    - _Requirements: 1.3, 1.4_

  - [x] 4.7 Write unit tests for sidecar classification and extraction
    - Test `_classify_document` with each document type's keywords and mixed keywords
    - Test `_extract_structured_fields` for cedula and contrato types
    - Test `/ocr/extract` endpoint with `document_type` parameter
    - Add to `ocr-service/test_main.py`
    - _Requirements: 2.1, 2.2, 3.1, 3.2_

- [x] 5. Checkpoint — Sidecar and backend integration
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Build reusable frontend components
  - [x] 6.1 Add frontend OCR extract types to `frontend/src/types/ocr.rs`
    - Add `OcrExtractResponse { document_type, fields, raw_lines }` and `OcrExtractField { name, value, label, confidence }` with `#[serde(rename_all = "camelCase")]`
    - Keep existing `ImportPreview`, `PreviewField`, `ConfirmPreviewRequest` types unchanged
    - _Requirements: 1.5_

  - [x] 6.2 Implement `OcrScanButton` component at `frontend/src/components/common/ocr_scan_button.rs`
    - Props: `document_type: AttrValue`, `on_result: Callback<Vec<OcrExtractField>>`, `label: AttrValue`, `disabled: bool`
    - Render button with provided label text
    - On click, open hidden `<input type="file" accept=".jpg,.jpeg,.png,.pdf">`
    - POST to `/api/v1/ocr/extract` as multipart with `file` + `document_type`
    - Show "Escaneando..." loading state while request is in flight, disable button
    - On success, call `on_result` with extracted fields
    - On error, show inline error message below button, auto-dismiss after 5 seconds using `Timeout` with cleanup closure
    - All text in Spanish
    - Re-export in `components/common/mod.rs`
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

  - [x] 6.3 Implement `ConfidenceInput` component at `frontend/src/components/common/confidence_input.rs`
    - Props: `value: AttrValue`, `confidence: Option<f64>`, `oninput: Callback<InputEvent>`, `input_type: AttrValue`, `placeholder: AttrValue`, `class: AttrValue`
    - confidence ≥ 0.7 or `None` → standard `gi-input` appearance
    - confidence < 0.7 → amber border (`#f59e0b`), light amber background (`#fffbeb`), badge showing "Confianza: NN%"
    - Re-export in `components/common/mod.rs`
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [x] 7. Integrate scan button into Pagos form
  - [x] 7.1 Add `OcrScanButton` to `frontend/src/pages/pagos.rs` create form
    - Add "📷 Escanear Recibo" button next to form title, only visible when creating (not editing)
    - Pass `document_type="deposito_bancario"` to `OcrScanButton`
    - On OCR result, set: `monto`, `moneda`, `fecha_pago`, `metodo_pago` = "transferencia", `notas` from `referencia` + `cuenta` concatenated
    - Do NOT overwrite `contrato_id`
    - Track per-field confidence in a `HashMap<String, f64>`, clear on manual edit
    - Wrap pre-filled fields with `ConfidenceInput` for fields with confidence < 0.7
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

  - [x] 7.2 Write property test for document_type passthrough (Property 4)
    - **Property 4: Provided document_type is used verbatim**
    - Generate random valid `document_type` from {deposito_bancario, recibo_gasto, cedula, contrato}; verify response `documentType` matches input regardless of content
    - Add to `backend/src/services/ocr_mapping_pbt.rs`
    - **Validates: Requirements 1.3**

  - [x] 7.3 Write property test for extract response structure (Property 5)
    - **Property 5: Extract response contains required structure**
    - Generate random `OcrResult`; verify response has non-empty `documentType`, `fields` array with correct field shapes, and `rawLines` array
    - **Validates: Requirements 1.5**

- [x] 8. Integrate scan button into Gastos form
  - [x] 8.1 Add `OcrScanButton` to `frontend/src/pages/gastos.rs` create form
    - Add "📷 Escanear Factura" button, only visible when creating (not editing)
    - Pass `document_type="recibo_gasto"` to `OcrScanButton`
    - On OCR result, set: `monto`, `moneda`, `fecha_gasto`, `proveedor`, `numero_factura`
    - Do NOT overwrite `propiedad_id`, `unidad_id`, or `categoria`
    - Track per-field confidence, wrap with `ConfidenceInput`
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 9. Integrate scan button into Inquilinos form
  - [x] 9.1 Add `OcrScanButton` to `frontend/src/pages/inquilinos.rs` create form
    - Add "📷 Escanear Cédula" button, only visible when creating (not editing)
    - Pass `document_type="cedula"` to `OcrScanButton`
    - On OCR result, set: `nombre`, `apellido`, `cedula`
    - Track per-field confidence, wrap with `ConfidenceInput`
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x] 10. Integrate scan button into Contratos form
  - [x] 10.1 Add `OcrScanButton` to `frontend/src/pages/contratos.rs` create form
    - Add "📷 Escanear Contrato" button, only visible when creating (not editing)
    - Pass `document_type="contrato"` to `OcrScanButton`
    - On OCR result, set: `monto_mensual`, `moneda`, `fecha_inicio`, `fecha_fin`, `deposito`
    - Do NOT overwrite `propiedad_id` or `inquilino_id`
    - Track per-field confidence, wrap with `ConfidenceInput`
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 11. Checkpoint — All form integrations
  - Ensure all tests pass, ask the user if questions arise.

- [x] 12. Add Pagos/Gastos options to import page
  - [x] 12.1 Update entity type dropdown in `frontend/src/pages/importar.rs`
    - Add "Pagos" and "Gastos" options to the entity type dropdown
    - When "Pagos" is selected with an image, use existing OCR preview flow via `/api/v1/importar/pagos`
    - When "Gastos" is selected with an image, use existing OCR preview flow via `/api/v1/importar/gastos`
    - When "Gastos" is selected with CSV/XLSX, use existing bulk import flow
    - _Requirements: 12.1, 12.2, 12.3, 12.4_

- [x] 13. Final checkpoint — Full feature verification
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation after each major layer (backend mappings, sidecar, frontend)
- Property tests use `proptest` (Rust) and `hypothesis` (Python) — both already available in the project
- The design is stateless: no data is persisted until the user submits the form
- Existing `map_deposito`/`map_gasto` logic is reused via new `_extract` variants that return `Vec<ExtractField>` instead of `ImportPreview`
