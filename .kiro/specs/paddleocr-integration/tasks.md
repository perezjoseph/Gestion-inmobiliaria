# Tasks

## Task 1: OCR Sidecar Service (Python FastAPI)

- [x] 1.1 Create `ocr-service/` directory with `requirements.txt` (fastapi, uvicorn, paddleocr, paddlepaddle, python-multipart, Pillow, PyMuPDF)
- [x] 1.2 Create `ocr-service/main.py` with FastAPI app: `GET /health` endpoint returning `{"status": "ok"}`, `POST /ocr/extract` accepting multipart image file
- [x] 1.3 Implement file validation in `POST /ocr/extract`: check format (JPEG, PNG, PDF), check size ≤ 10 MB, return 422 for unsupported format, 413 for oversized
- [x] 1.4 Implement OCR extraction: load PP-OCRv5 for text recognition, run on uploaded image, collect text lines with bounding boxes and confidence scores
- [x] 1.5 Implement document type classification: keyword heuristics on extracted text (e.g., "DEPOSITO"/"AHORROS" → `deposito_bancario`, "FACTURA"/"RECIBO" → `recibo_gasto`)
- [x] 1.6 Implement structured field extraction based on document type: for `deposito_bancario` extract monto/moneda/fecha/depositante/cuenta/referencia; for `recibo_gasto` extract proveedor/monto/moneda/fecha/numero_factura
- [x] 1.7 Return `OcrResult` JSON response with `document_type`, `lines` array, and `structured_fields` object
- [x] 1.8 Create `ocr-service/Dockerfile`: Python base image, install dependencies, pre-download PaddleOCR models during build, expose port 8000
- [x] 1.9 Write pytest tests for sidecar endpoints (valid image → 200, unsupported format → 422, oversized → 413, health → 200)

## Task 2: Docker Compose and Configuration

- [x] 2.1 Add `ocr-service` to `docker-compose.prod.yml` with build context `./ocr-service`, healthcheck on `/health` (interval 30s, start_period 60s), security_opt, read_only, tmpfs
- [x] 2.2 Update `backend` service in `docker-compose.prod.yml`: add `depends_on` for `ocr-service` with `condition: service_healthy`, add `OCR_SERVICE_URL: http://ocr-service:8000` environment variable
- [x] 2.3 Add `OCR_SERVICE_URL=http://localhost:8000` to `.env.example`

## Task 3: Backend OCR Data Models

- [x] 3.1 Create `backend/src/models/ocr.rs` with `OcrResult`, `OcrLine`, `ImportPreview`, `PreviewField`, `ConfirmPreviewRequest` structs (serde Serialize/Deserialize, camelCase for API-facing types)
- [x] 3.2 Re-export `ocr` module in `backend/src/models/mod.rs`
- [x] 3.3 Add `Image` variant to `ImportFormat` enum in `backend/src/models/importacion.rs`

## Task 4: Backend OCR Client

- [x] 4.1 Add `reqwest` (with `multipart` feature) and `dashmap` dependencies to `backend/Cargo.toml`
- [x] 4.2 Create `backend/src/services/ocr_client.rs` with `OcrClient` struct: constructor reading `OCR_SERVICE_URL` env var, `extract` method sending multipart to sidecar, 30s timeout, error handling returning `AppError::Internal`
- [x] 4.3 Re-export `ocr_client` module in `backend/src/services/mod.rs`

## Task 5: DR Date and Currency Parsing

- [x] 5.1 Create `backend/src/services/ocr_mapping.rs` with `parse_dr_date` function: parse DD-MM-YY, DD/MM/YYYY, DD-MM-YYYY, YYYY-MM-DD formats, two-digit year rule (00-49 → 2000-2049, 50-99 → 1950-1999)
- [x] 5.2 Implement `parse_dr_currency` function: recognize RD$ → DOP, US$ → USD, no prefix → DOP default, strip comma thousands separators, parse decimal amount
- [x] 5.3 Re-export `ocr_mapping` module in `backend/src/services/mod.rs`

## Task 6: Field Mapping (Deposit and Expense)

- [x] 6.1 Implement `map_deposito` in `ocr_mapping.rs`: extract fields from `OcrResult.structured_fields`, call `parse_dr_date`/`parse_dr_currency`, build `PreviewPago` with metodo_pago="deposito_bancario" and estado="pagado", map cuenta+referencia to notas
- [x] 6.2 Implement `map_gasto` in `ocr_mapping.rs`: extract fields from `OcrResult.structured_fields`, call `parse_dr_date`/`parse_dr_currency`, build `PreviewGasto`, return error with "monto no detectado" if amount missing

## Task 7: Preview Store

- [x] 7.1 Create `backend/src/services/ocr_preview.rs` with `PreviewStore` struct using `DashMap<Uuid, (ImportPreview, Instant)>`, implement `insert`, `get`, `remove`, `cleanup_expired` methods with 30-minute TTL
- [x] 7.2 Register `PreviewStore` as `web::Data<PreviewStore>` in `backend/src/app.rs` (or `main.rs`), spawn background `tokio::spawn` task running `cleanup_expired` every 5 minutes
- [x] 7.3 Re-export `ocr_preview` module in `backend/src/services/mod.rs`

## Task 8: Import Handler Extensions

- [x] 8.1 Update `detect_format` in `backend/src/handlers/importacion.rs` to return `ImportFormat::Image` for `.jpg`, `.jpeg`, `.png`, `.pdf` extensions
- [x] 8.2 Update `importar_pagos` and `importar_gastos` handlers: when `ImportFormat::Image` detected, call `OcrClient::extract`, then `map_deposito`/`map_gasto`, store in `PreviewStore`, return `ImportPreview` JSON response
- [x] 8.3 Add `confirmar_preview` handler at `POST /api/v1/importar/ocr/confirmar`: read preview from store, apply corrections, create pago or gasto record, remove preview
- [x] 8.4 Add `descartar_preview` handler at `DELETE /api/v1/importar/ocr/preview/{preview_id}`: remove preview from store, return 200 or 404
- [x] 8.5 Register new routes in `backend/src/routes.rs` under `/importar` scope

## Task 9: Frontend OCR Types and API

- [x] 9.1 Create `frontend/src/types/ocr.rs` with `ImportPreview` and `PreviewField` types, re-export in `frontend/src/types/mod.rs`
- [x] 9.2 Add API functions in `frontend/src/services/api.rs`: `confirmar_preview(request)` calling `POST /api/v1/importar/ocr/confirmar`, `descartar_preview(preview_id)` calling `DELETE /api/v1/importar/ocr/preview/{preview_id}`

## Task 10: Frontend Import Page Updates

- [x] 10.1 Update import page file input to accept `.jpg,.jpeg,.png,.pdf` in addition to `.csv,.xlsx`
- [x] 10.2 Add file type indicator component (`frontend/src/components/importacion/file_type_indicator.rs`) showing spreadsheet vs image icon/label
- [x] 10.3 Create OCR preview component (`frontend/src/components/importacion/ocr_preview.rs`): editable form with field labels in Spanish, confidence scores, amber highlight for confidence < 0.7, "Confirmar" and "Descartar" buttons
- [x] 10.4 Update import page to detect `preview_id` in response and render `OcrPreview` component instead of standard result
- [x] 10.5 Wire "Confirmar" button to call `confirmar_preview` API, display standard import result on success
- [x] 10.6 Wire "Descartar" button to call `descartar_preview` API, reset import page state
- [x] 10.7 Re-export new components in `frontend/src/components/importacion/mod.rs`

## Task 11: Property-Based Tests

- [x] 11.1 Create `backend/tests/ocr_pbt.rs` with proptest: OcrResult serialization round-trip (Property 1) — generate random OcrResult with Unicode text, serialize to JSON, deserialize, assert equality
- [x] 11.2 Add proptest: DR date parsing round-trip (Property 2) — generate random NaiveDate in 1950-2049, format in each supported format, parse with `parse_dr_date`, assert original date
- [x] 11.3 Add proptest: DR currency parsing round-trip (Property 3) — generate random non-negative Decimal (≤2 decimal places), format with RD$/US$/no prefix and comma separators, parse with `parse_dr_currency`, assert original amount and correct currency code
- [x] 11.4 Add proptest: file extension detection (Property 4) — generate random filenames with known extensions, verify `detect_format` returns correct `ImportFormat` variant
- [x] 11.5 Add proptest: deposit receipt field mapping completeness (Property 5) — generate random OcrResult with deposito_bancario type and valid structured_fields, verify `map_deposito` produces correct PreviewPago
- [x] 11.6 Add proptest: expense receipt field mapping completeness (Property 6) — generate random OcrResult with recibo_gasto type and valid structured_fields, verify `map_gasto` produces correct PreviewGasto
- [x] 11.7 Add proptest: preview store TTL expiry (Property 7) — insert preview, simulate time past TTL, run cleanup, verify get returns None

## Task 12: Unit and Integration Tests

- [x] 12.1 Add unit tests in `backend/tests/ocr_tests.rs`: `parse_dr_date` with concrete examples for each format, `parse_dr_currency` with concrete examples, `detect_format` for all extensions, `map_deposito` constant fields, `map_gasto` missing monto error
- [x] 12.2 Add unit tests for `OcrClient` error handling: mock server returning 500 → `AppError::Internal`, mock server returning invalid JSON → `AppError::Internal`
- [x] 12.3 Add unit tests for `PreviewStore`: insert/get round-trip, remove deletes, get after remove returns None
