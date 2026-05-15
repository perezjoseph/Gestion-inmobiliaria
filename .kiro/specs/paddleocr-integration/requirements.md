# Requirements Document

## Introduction

Add OCR (Optical Character Recognition) capability to the existing bulk import system so that property managers can upload photos of Dominican Republic bank deposit receipts, expense receipts, and other documents. A Python FastAPI sidecar service running PaddleOCR extracts text from images, and the Rust backend maps the extracted fields to the application's existing data structures (pagos, gastos). The primary use case is scanning Banco Popular deposit receipts (Deposito-Ahorros) to create payment records, with extensibility to other document types.

## Glossary

- **OCR_Sidecar**: A Python FastAPI microservice running PaddleOCR (PP-OCRv5 for text recognition, PP-StructureV3 for document structure parsing) that accepts image uploads and returns structured extracted text. Runs as a Docker Compose service on CPU.
- **Backend**: The Rust Actix-web application that handles API requests, calls the OCR_Sidecar via HTTP, and persists data to PostgreSQL.
- **Frontend**: The Yew WASM single-page application that provides the user interface for uploading files.
- **Import_Handler**: The Actix-web multipart handler at `POST /api/v1/importar/{entity_type}` that receives uploaded files and delegates processing.
- **OCR_Client**: A Rust module using `reqwest` to make HTTP calls from the Backend to the OCR_Sidecar.
- **OCR_Result**: A structured JSON response from the OCR_Sidecar containing extracted fields, confidence scores, and the detected document type.
- **Document_Type**: A classification label assigned by the OCR_Sidecar indicating the kind of document scanned (e.g., "deposito_bancario", "recibo_gasto", "cedula", "contrato").
- **Field_Mapping**: The process of converting OCR_Result fields into the application's existing row-based import format (matching columns like monto, fecha_pago, moneda, referencia).
- **Confidence_Score**: A numeric value between 0.0 and 1.0 indicating how certain the OCR engine is about a recognized text region.
- **Import_Preview**: A UI step where the user reviews OCR-extracted data before confirming the import, allowing manual corrections.

## Requirements

### Requirement 1: OCR Sidecar Service

**User Story:** As a system operator, I want a self-contained OCR microservice, so that the Rust backend can offload image text extraction without bundling Python dependencies.

#### Acceptance Criteria

1. THE OCR_Sidecar SHALL expose a `POST /ocr/extract` endpoint that accepts a multipart image file and returns an OCR_Result as JSON.
2. THE OCR_Sidecar SHALL use PaddleOCR PP-OCRv5 for text line recognition and PP-StructureV3 for document structure parsing.
3. THE OCR_Sidecar SHALL accept images in JPEG, PNG, and PDF formats with a maximum file size of 10 MB.
4. WHEN the OCR_Sidecar receives a valid image, THE OCR_Sidecar SHALL return a JSON response containing: a list of extracted text lines with bounding box coordinates, a Confidence_Score per line, and a detected Document_Type.
5. WHEN the OCR_Sidecar receives an unsupported file format, THE OCR_Sidecar SHALL return HTTP 422 with a descriptive error message.
6. WHEN the OCR_Sidecar receives a file exceeding 10 MB, THE OCR_Sidecar SHALL return HTTP 413 with a descriptive error message.
7. THE OCR_Sidecar SHALL expose a `GET /health` endpoint that returns HTTP 200 when the service and PaddleOCR models are loaded and ready.
8. THE OCR_Sidecar SHALL run on CPU without requiring GPU hardware.
9. THE OCR_Sidecar SHALL be defined as a service in `docker-compose.prod.yml` with a health check on the `/health` endpoint.
10. THE OCR_Sidecar SHALL process a single-page document image within 10 seconds on a standard 2-core CPU.

### Requirement 2: Backend OCR Client Integration

**User Story:** As a backend developer, I want the Rust backend to call the OCR sidecar via HTTP, so that image uploads are processed without adding Python to the Rust build.

#### Acceptance Criteria

1. THE Backend SHALL include `reqwest` as a dependency for making HTTP requests to the OCR_Sidecar.
2. THE OCR_Client SHALL be implemented as a module at `backend/src/services/ocr_client.rs` that sends multipart image data to the OCR_Sidecar `POST /ocr/extract` endpoint and deserializes the OCR_Result response.
3. THE OCR_Client SHALL read the OCR_Sidecar base URL from the `OCR_SERVICE_URL` environment variable.
4. THE OCR_Client SHALL set a request timeout of 30 seconds when calling the OCR_Sidecar.
5. IF the OCR_Sidecar is unreachable or returns a non-2xx status, THEN THE OCR_Client SHALL return an `AppError::Internal` with a message indicating the OCR service failure.
6. IF the OCR_Sidecar returns a response that cannot be deserialized into an OCR_Result, THEN THE OCR_Client SHALL return an `AppError::Internal` with a message describing the parse failure.

### Requirement 3: Import Format Extension for Images

**User Story:** As a property manager, I want to upload images through the same import page I already use for CSV/XLSX, so that I have a single workflow for all data imports.

#### Acceptance Criteria

1. THE Backend SHALL extend the `ImportFormat` enum with an `Image` variant to represent image-based imports.
2. WHEN the Import_Handler receives a file with extension `.jpg`, `.jpeg`, `.png`, or `.pdf`, THE Import_Handler SHALL detect the format as `ImportFormat::Image`.
3. WHEN the Import_Handler detects `ImportFormat::Image`, THE Import_Handler SHALL send the file to the OCR_Client instead of the CSV/XLSX parser.
4. THE Import_Handler SHALL continue to handle `.csv` and `.xlsx` files using the existing CSV and XLSX parsers without modification.

### Requirement 4: Deposit Receipt Field Mapping (Pagos)

**User Story:** As a property manager, I want deposit receipt photos automatically mapped to payment records, so that I can record rent payments without manual data entry.

#### Acceptance Criteria

1. WHEN the OCR_Result Document_Type is "deposito_bancario", THE Backend SHALL extract the following fields from the OCR_Result: transaction amount, currency, transaction date, depositor name, description/reference, and account number.
2. THE Field_Mapping SHALL map the extracted transaction amount to `pago.monto` and the currency indicator (RD$ or US$) to `pago.moneda` ("DOP" or "USD").
3. THE Field_Mapping SHALL map the extracted transaction date to `pago.fecha_pago` as a `NaiveDate`.
4. THE Field_Mapping SHALL map the extracted account number to `pago.notas` as a reference identifier.
5. THE Field_Mapping SHALL map the extracted description text to `pago.notas`, appending it to any account number reference.
6. WHEN the extracted depositor name matches an existing inquilino by `nombre` and `apellido` (case-insensitive partial match), THE Backend SHALL look up the active contrato for that inquilino and set `pago.contrato_id` accordingly.
7. IF the depositor name does not match any existing inquilino, THEN THE Backend SHALL include the unmatched name in the Import_Preview for manual tenant selection.
8. THE Field_Mapping SHALL set `pago.metodo_pago` to "deposito_bancario".
9. THE Field_Mapping SHALL set `pago.estado` to "pagado" for deposit receipts.

### Requirement 5: Expense Receipt Field Mapping (Gastos)

**User Story:** As a property manager, I want to scan expense receipts and have them mapped to expense records, so that I can track property expenses from physical receipts.

#### Acceptance Criteria

1. WHEN the OCR_Result Document_Type is "recibo_gasto", THE Backend SHALL extract the following fields: vendor/provider name, total amount, currency, date, and invoice/receipt number.
2. THE Field_Mapping SHALL map the extracted vendor name to `gasto.proveedor`.
3. THE Field_Mapping SHALL map the extracted total amount to `gasto.monto` and the currency indicator to `gasto.moneda`.
4. THE Field_Mapping SHALL map the extracted date to `gasto.fecha_gasto` as a `NaiveDate`.
5. THE Field_Mapping SHALL map the extracted invoice number to `gasto.numero_factura`.
6. IF the OCR_Result does not contain a recognizable amount field, THEN THE Backend SHALL return the extracted text in the Import_Preview with an error indicating "monto no detectado".

### Requirement 6: OCR Import Preview and Confirmation

**User Story:** As a property manager, I want to review OCR-extracted data before it is saved, so that I can correct any recognition errors.

#### Acceptance Criteria

1. WHEN the Backend processes an image import, THE Backend SHALL return an Import_Preview response containing the extracted fields, Confidence_Scores, and the detected Document_Type instead of immediately persisting the data.
2. THE Import_Preview response SHALL include a `preview_id` (UUID) that the Frontend uses to confirm or discard the import.
3. THE Backend SHALL expose a `POST /api/v1/importar/ocr/confirmar` endpoint that accepts a `preview_id` and optionally corrected field values, then persists the record.
4. THE Backend SHALL expose a `DELETE /api/v1/importar/ocr/preview/{preview_id}` endpoint that discards a pending preview.
5. WHILE an Import_Preview is pending, THE Backend SHALL store the preview data in a temporary storage mechanism with a time-to-live of 30 minutes.
6. IF a preview is not confirmed within 30 minutes, THEN THE Backend SHALL automatically discard the preview data.
7. WHEN the user confirms an Import_Preview for a "deposito_bancario" document, THE Backend SHALL create a pago record using the confirmed field values.
8. WHEN the user confirms an Import_Preview for a "recibo_gasto" document, THE Backend SHALL create a gasto record using the confirmed field values.

### Requirement 7: Frontend Image Upload Support

**User Story:** As a property manager, I want the import page to accept image files alongside CSV/XLSX, so that I can upload receipt photos from the same interface.

#### Acceptance Criteria

1. THE Frontend import page SHALL accept file types `.jpg`, `.jpeg`, `.png`, `.pdf` in addition to `.csv` and `.xlsx`.
2. THE Frontend SHALL display a file type indicator showing whether the selected file is a spreadsheet or an image.
3. WHEN the Backend returns an Import_Preview response (detected by the presence of a `preview_id` field), THE Frontend SHALL display a preview form showing the extracted fields with their Confidence_Scores.
4. THE Frontend SHALL highlight fields with a Confidence_Score below 0.7 in a warning color to indicate low-confidence extractions.
5. THE Frontend SHALL allow the user to edit any extracted field value in the preview form before confirming.
6. THE Frontend SHALL provide "Confirmar" and "Descartar" buttons on the preview form that call the confirm and discard endpoints respectively.
7. WHEN the user confirms the preview, THE Frontend SHALL display the standard import result (total_filas, exitosos, fallidos).
8. THE Frontend SHALL display all visible text in Spanish, including field labels, buttons, status messages, and error messages.

### Requirement 8: Docker Compose and Configuration

**User Story:** As a system operator, I want the OCR sidecar integrated into the existing Docker Compose setup, so that deployment requires no additional manual steps.

#### Acceptance Criteria

1. THE OCR_Sidecar SHALL be defined in `docker-compose.prod.yml` as a service named `ocr-service`.
2. THE Backend service in `docker-compose.prod.yml` SHALL declare a dependency on `ocr-service` with condition `service_healthy`.
3. THE Backend service SHALL receive the `OCR_SERVICE_URL` environment variable pointing to `http://ocr-service:8000`.
4. THE OCR_Sidecar service SHALL have a health check configured using the `GET /health` endpoint with an interval of 30 seconds and a start period of 60 seconds.
5. THE `.env.example` file SHALL include the `OCR_SERVICE_URL` variable with a default value of `http://localhost:8000`.
6. THE OCR_Sidecar SHALL have a `Dockerfile` at `ocr-service/Dockerfile` that installs Python, PaddleOCR, and FastAPI dependencies.
7. THE OCR_Sidecar Docker image SHALL pre-download PaddleOCR models during the build phase so that container startup does not require internet access.

### Requirement 9: OCR Response Serialization Round-Trip

**User Story:** As a backend developer, I want OCR response parsing to be reliable, so that no data is lost or corrupted between the sidecar and the Rust backend.

#### Acceptance Criteria

1. THE OCR_Sidecar SHALL serialize OCR_Result as JSON following a documented schema with fields: `document_type` (string), `lines` (array of objects with `text`, `confidence`, `bbox`), and `structured_fields` (object with key-value pairs of extracted named fields).
2. THE OCR_Client SHALL deserialize the JSON response into a Rust `OcrResult` struct using `serde`.
3. FOR ALL valid OcrResult values, serializing to JSON then deserializing back SHALL produce an equivalent OcrResult (round-trip property).
4. THE OCR_Client SHALL preserve numeric precision for Confidence_Score values to at least two decimal places.
5. THE OCR_Client SHALL preserve all Unicode characters in extracted text fields, including accented Spanish characters (á, é, í, ó, ú, ñ, ü).

### Requirement 10: Dominican Republic Date and Currency Parsing

**User Story:** As a property manager working in the DR, I want the system to correctly parse Dominican date formats and currency notations from receipts, so that extracted data matches local conventions.

#### Acceptance Criteria

1. THE Field_Mapping SHALL parse dates in the formats DD-MM-YY, DD/MM/YYYY, DD-MM-YYYY, and YYYY-MM-DD from OCR-extracted text.
2. WHEN a two-digit year is encountered, THE Field_Mapping SHALL interpret years 00-49 as 2000-2049 and years 50-99 as 1950-1999.
3. THE Field_Mapping SHALL recognize "RD$" as DOP currency and "US$" as USD currency.
4. THE Field_Mapping SHALL parse monetary amounts that use commas as thousands separators and periods as decimal separators (e.g., "RD$50,000.00" parses to monto=50000.00, moneda="DOP").
5. IF the OCR-extracted text contains no recognizable currency prefix, THEN THE Field_Mapping SHALL default to "DOP".
6. FOR ALL valid date strings in the supported formats, parsing then formatting back to the same format SHALL produce the original string (round-trip property for date parsing).
7. FOR ALL valid monetary amount strings, parsing then formatting back SHALL produce a semantically equivalent string (round-trip property for currency parsing).
