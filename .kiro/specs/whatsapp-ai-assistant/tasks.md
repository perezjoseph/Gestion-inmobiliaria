# Implementation Plan: WhatsApp AI Assistant

## Overview

Implements a per-organization WhatsApp AI chatbot using a Node.js sidecar (Baileys) for WhatsApp connectivity and a Rust AI module (Rig + OVMS) for conversational intelligence. The implementation proceeds bottom-up: database entities → core services → AI module → handlers → frontend wizard → integration wiring.

## Tasks

- [x] 1. Database entities and migration
  - [x] 1.1 Create the chatbot migration file
    - Create `backend/migration/src/m20260509_000001_create_chatbot_tables.rs`
    - Define `chatbot_config` table with all columns (id, organizacion_id unique FK, activo, connection_status, display_name, language, tone, greeting, system_prompt, faqs JSONB, policies, sender_policy, allowlist JSONB, capabilities JSONB, handoff_keywords JSONB, history_limit, retention_days, updated_by FK, timestamps)
    - Define `chatbot_conversation` table with columns (id, organizacion_id FK, sender_phone, inquilino_id nullable FK, role, content, message_type, metadata JSONB nullable, created_at)
    - Add composite index `(organizacion_id, sender_phone, created_at DESC)` and `(organizacion_id, created_at)`
    - Define `chatbot_receipt_extraction` table with columns (id, organizacion_id FK, conversation_id FK, inquilino_id nullable FK, contrato_id nullable FK, extracted_data JSONB, status, confirmed_by nullable FK, timestamps)
    - Register migration in `backend/migration/src/lib.rs`
    - _Requirements: 7.1, 7.3, 4.7, 8.6_

  - [x] 1.2 Create SeaORM entities for chatbot tables
    - Create `backend/src/entities/chatbot_config.rs` with Model, ActiveModel, Relation (belongs to organizacion)
    - Create `backend/src/entities/chatbot_conversation.rs` with Model, ActiveModel, Relations
    - Create `backend/src/entities/chatbot_receipt_extraction.rs` with Model, ActiveModel, Relations
    - Register all three in `backend/src/entities/mod.rs` and `prelude.rs`
    - _Requirements: 7.1, 4.7, 8.1_

  - [x] 1.3 Create chatbot DTOs (request/response models)
    - Create `backend/src/models/chatbot.rs` with structs: `ChatbotConfigResponse`, `ChatbotConfigUpdateRequest`, `IncomingWebhookPayload`, `SendMessageRequest`, `ConversationListResponse`, `ReceiptExtractionResponse`, `ReceiptConfirmRequest`, `TestChatRequest`, `TestChatResponse`, `ConnectionStatusResponse`
    - Add validation annotations (display_name 1–100 chars, tone 1–50 chars, FAQ limits, policies max 5000 chars)
    - Register in `backend/src/models/mod.rs`
    - _Requirements: 9.2, 9.4, 9.5, 9.9, 10.1_

- [x] 2. Core backend services
  - [x] 2.1 Implement chatbot configuration service
    - Create `backend/src/services/chatbot.rs`
    - Implement `get_config(org_id)` — load or return default config
    - Implement `upsert_config(org_id, update_request, user_id)` — validate and persist
    - Implement `validate_config()` — enforce display_name length, FAQ count/length limits, policies length, sender_policy enum values
    - Enforce `admin` or `gerente` role check at service level
    - _Requirements: 9.2, 9.3, 9.4, 9.5, 9.6, 9.8, 9.9_

  - [x] 2.2 Write property test for configuration round-trip
    - **Property 17: Configuration Round-Trip**
    - **Validates: Requirements 9.2, 9.3, 9.4, 9.5**

  - [x] 2.3 Implement sender policy engine
    - Add `is_sender_allowed(config, phone, org_id, db)` to chatbot service
    - Implement `tenants_only` — query inquilinos by phone + org_id
    - Implement `tenants_and_prospects` — always allow
    - Implement `allowlist` — check config.allowlist contains phone
    - Default deny for unrecognized policy values (fail-closed)
    - _Requirements: 2.1, 2.2, 2.3, 2.5, 2.6_

  - [x] 2.4 Write property tests for sender policy
    - **Property 2: Sender Policy Correctness**
    - **Validates: Requirements 2.1, 2.2, 2.3**

  - [x] 2.5 Write property test for organization-scoped tenant resolution
    - **Property 3: Organization-Scoped Tenant Resolution Isolation**
    - **Validates: Requirements 2.5, 13.2**

  - [x] 2.6 Implement phone number validation and normalization
    - Add `validate_e164(phone)` — check against `^\+[1-9]\d{1,14}$`
    - Add `normalize_phone(input, default_country_code)` — strip non-digits, remove trunk zeros, prepend country code
    - Reject numbers that cannot be normalized (fewer than 7 digits or invalid chars)
    - _Requirements: 13.1, 13.3, 13.4_

  - [x] 2.7 Write property test for phone number E.164 validation
    - **Property 21: Phone Number E.164 Validation**
    - **Validates: Requirements 13.1, 13.3**

  - [x] 2.8 Implement conversation history service
    - Add `persist_message(org_id, sender_phone, inquilino_id, role, content, message_type, metadata)` to chatbot service
    - Add `load_history(org_id, sender_phone, limit)` — query last N messages ordered by created_at DESC
    - Add `cleanup_expired(org_id, retention_days)` — delete messages older than retention period
    - _Requirements: 7.1, 7.2, 7.4, 7.5_

  - [x] 2.9 Write property test for conversation history windowing
    - **Property 6: Conversation History Windowing**
    - **Validates: Requirements 3.2, 7.2**

  - [x] 2.10 Write property test for retention cleanup correctness
    - **Property 14: Retention Cleanup Correctness**
    - **Validates: Requirement 7.4**

- [x] 3. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Receipt extraction and balance services
  - [x] 4.1 Implement confidence level mapping
    - Add `Confidence` enum (High, Medium, Low) to chatbot models
    - Implement `map_confidence(score: f64) -> Confidence` with thresholds: ≥0.85 High, 0.60–0.84 Medium, <0.60 Low
    - _Requirements: 4.2_

  - [x] 4.2 Write property test for confidence level mapping
    - **Property 8: Confidence Level Mapping**
    - **Validates: Requirement 4.2**

  - [x] 4.3 Implement receipt extraction service
    - Add `record_extraction(org_id, conversation_id, inquilino_id, contrato_id, extracted_data, confidence)` to chatbot service
    - Route based on confidence: High + resolved tenant → `pending_confirmation`; Medium/Low → always queue for landlord confirmation
    - Store in `chatbot_receipt_extraction` table with originating conversation reference
    - _Requirements: 4.3, 4.4, 4.5, 4.7_

  - [x] 4.4 Write property test for confidence-based receipt routing
    - **Property 9: Confidence-Based Receipt Routing**
    - **Validates: Requirements 4.3, 4.4**

  - [x] 4.5 Implement receipt confirmation workflow
    - Add `list_pending_receipts(org_id)` — query extractions with status `pending_confirmation` ordered by created_at DESC
    - Add `confirm_receipt(extraction_id, user_id)` — set status `confirmed`, record confirming user, create Pago record
    - Add `reject_receipt(extraction_id, user_id, reason)` — set status `rejected`, persist reason
    - Enforce atomic status transitions (reject if not in `pending_confirmation`)
    - _Requirements: 8.1, 8.2, 8.3, 8.5, 8.6_

  - [x] 4.6 Write property tests for receipt status transitions
    - **Property 15: Pending Receipts Visibility**
    - **Property 16: Receipt Status Transitions**
    - **Validates: Requirements 8.1, 8.2, 8.3**

  - [x] 4.7 Implement balance query logic
    - Add `query_tenant_balance(inquilino_id, org_id)` — sum payments with status `pendiente` or `atrasado` grouped by currency
    - Format amounts with currency symbol (RD$ for DOP, US$ for USD) and two decimal places
    - Return individual payment details (amount, currency, due date) plus totals per currency
    - _Requirements: 5.1, 5.3, 5.4_

  - [x] 4.8 Write property tests for balance and currency formatting
    - **Property 10: Balance Calculation Correctness**
    - **Property 11: Currency Formatting**
    - **Validates: Requirements 5.1, 5.3**

  - [x] 4.9 Implement maintenance request creation via chatbot
    - Add `create_maintenance_from_chat(inquilino_id, org_id, description, priority)` — create SolicitudMantenimiento linked to tenant's active contract property
    - Default status `pendiente`, default priority `media` unless specified
    - Validate description length (2–1000 chars)
    - Handle multiple active contracts: use most recently started contract's property as fallback
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [x] 4.10 Write property test for maintenance request defaults
    - **Property 12: Maintenance Request Defaults and Linking**
    - **Validates: Requirements 6.1, 6.2**

- [x] 5. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. AI Module (Rig integration)
  - [x] 6.1 Create AI module structure and OVMS client
    - Create `backend/src/services/ai_module.rs`
    - Define `AiModule` struct with `rig::providers::openai::Client` pointed at OVMS endpoint
    - Configure client with `OVMS_ENDPOINT`, `OVMS_CHAT_MODEL`, and `AI_CHAT_TIMEOUT_SECS` from env/config
    - Add new env vars to `backend/src/config.rs`: `BAILEYS_SERVICE_URL`, `BAILEYS_INTERNAL_TOKEN`, `OVMS_ENDPOINT`, `OVMS_CHAT_MODEL`, `AI_CHAT_TIMEOUT_SECS`
    - _Requirements: 3.3, 10.4, 10.5_

  - [x] 6.2 Implement system prompt composition
    - Add `compose_system_prompt(config, tenant_context, faqs, policies, handoff_keywords)` to ai_module
    - Include: tone keyword, greeting, all FAQ Q&A pairs, policies text, tenant name (when resolved), handoff keywords
    - _Requirements: 3.1, 11.3_

  - [x] 6.3 Write property test for system prompt composition completeness
    - **Property 5: System Prompt Composition Completeness**
    - **Validates: Requirement 3.1**

  - [x] 6.4 Implement Rig tool definitions
    - Define `ExtractReceiptTool` — Rig `Tool` impl that calls OCR service and maps to `PaymentReceipt`
    - Define `QueryBalanceTool` — Rig `Tool` impl that calls `query_tenant_balance`
    - Define `CreateMaintenanceRequestTool` — Rig `Tool` impl that calls `create_maintenance_from_chat`
    - Define `GetPaymentHistoryTool` — Rig `Tool` impl that queries recent pagos for tenant
    - Define `HandoffToHumanTool` — Rig `Tool` impl that flags conversation for human handoff
    - Each tool receives typed input from LLM and calls the appropriate backend service
    - _Requirements: 3.5, 4.1, 5.1, 6.1, 11.1_

  - [x] 6.5 Implement capability-gated tool registration
    - Add `build_agent(config)` — register tools only if corresponding capability is enabled in `config.capabilities`
    - Map: `receipt_ocr` → ExtractReceiptTool, `balance_queries` → QueryBalanceTool, `maintenance_requests` → CreateMaintenanceRequestTool, `human_handoff` → HandoffToHumanTool
    - GetPaymentHistoryTool registered when `balance_queries` is enabled
    - _Requirements: 3.5, 3.6, 11.6_

  - [x] 6.6 Write property test for tool registration matching capabilities
    - **Property 7: Tool Registration Matches Capabilities**
    - **Validates: Requirements 3.5, 3.6**

  - [x] 6.7 Implement `process_message` entry point
    - Implement `AiModule::process_message(config, tenant_context, history, user_message)` → `AgentResponse`
    - Compose system prompt, build agent with enabled tools, pass history + message
    - Handle OVMS timeout (30s) — return error message to sender
    - Handle tool invocation results and compose final reply
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.7_

- [x] 7. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Internal webhook and message pipeline
  - [x] 8.1 Implement internal webhook handler
    - Create `backend/src/handlers/chatbot_internal.rs`
    - POST `/internal/whatsapp/incoming` — parse `IncomingWebhookPayload`
    - Validate `X-Internal-Token` header using constant-time comparison against `BAILEYS_INTERNAL_TOKEN`
    - Reject with HTTP 401 if token missing/invalid (log rejection without logging token value)
    - Register in routes
    - _Requirements: 10.1, 10.2, 10.3_

  - [x] 8.2 Write property test for internal webhook authentication
    - **Property 19: Internal Webhook Authentication**
    - **Validates: Requirements 10.1, 10.2**

  - [x] 8.3 Implement message processing pipeline
    - In webhook handler: load ChatbotConfig → check `activo` → apply sender policy → resolve tenant → load history → invoke AI module → persist messages → send reply via Baileys
    - If `activo` is false, silently discard without reply
    - If sender not authorized, silently discard without reply (log for monitoring)
    - If AI fails, persist user message and send error reply
    - _Requirements: 2.4, 3.4, 7.1, 7.6, 9.7_

  - [x] 8.4 Write property test for unauthorized senders not invoking AI
    - **Property 4: Unauthorized Senders Do Not Invoke AI**
    - **Validates: Requirements 2.4, 9.7**

  - [x] 8.5 Implement human handoff state management
    - Add `handoff_status` tracking (per org + sender_phone) — `none` or `awaiting_human`
    - In message pipeline: if handoff active, persist message but skip AI invocation
    - Add `clear_handoff(org_id, sender_phone)` for admin/gerente to resume AI
    - _Requirements: 11.1, 11.2, 11.4, 11.5_

  - [x] 8.6 Write property test for handoff ceasing AI responses
    - **Property 20: Handoff Ceases AI Responses**
    - **Validates: Requirement 11.3**

  - [x] 8.7 Write property test for conversation persistence completeness
    - **Property 13: Conversation Persistence Completeness**
    - **Validates: Requirement 7.1**

- [x] 9. Public API handlers
  - [x] 9.1 Implement chatbot configuration handlers
    - Create `backend/src/handlers/chatbot.rs`
    - GET `/api/v1/chatbot/config` — return config for current org
    - PUT `/api/v1/chatbot/config` — create/update config (validate, persist, audit)
    - Enforce `admin` or `gerente` role via existing RBAC extractors
    - Register routes in `backend/src/routes.rs`
    - _Requirements: 9.1, 9.2, 9.6, 9.8_

  - [x] 9.2 Write property test for role-based access control
    - **Property 18: Role-Based Access Control for Configuration**
    - **Validates: Requirement 9.6**

  - [x] 9.3 Implement connection management handlers
    - POST `/api/v1/chatbot/connect` — call Baileys Service start session, return QR or status
    - POST `/api/v1/chatbot/disconnect` — call Baileys Service stop session
    - GET `/api/v1/chatbot/status` — call Baileys Service get status
    - _Requirements: 1.1, 1.2, 1.3_

  - [x] 9.4 Implement conversation and receipt handlers
    - GET `/api/v1/chatbot/conversations` — list recent conversations (paginated)
    - GET `/api/v1/chatbot/conversations/{phone}` — get history for a phone
    - GET `/api/v1/chatbot/receipts/pending` — list pending receipt confirmations
    - POST `/api/v1/chatbot/receipts/{id}/confirm` — confirm extraction
    - POST `/api/v1/chatbot/receipts/{id}/reject` — reject extraction
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

  - [x] 9.5 Implement test chat handler
    - POST `/api/v1/chatbot/test` — process message through full AI pipeline without Baileys delivery
    - Use current (possibly unsaved) config from request body
    - Do NOT persist test messages in production conversation table
    - Return AI response or error indication
    - _Requirements: 12.1, 12.3, 12.4, 12.5_

- [x] 10. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 11. Baileys Service (Node.js sidecar)
  - [x] 11.1 Initialize Baileys Service project
    - Create `baileys-service/` directory with `package.json`, `tsconfig.json`
    - Dependencies: `@whiskeysockets/baileys`, `express`, `qrcode`, `pino`
    - Create `baileys-service/src/index.ts` — Express server on port 3100
    - Add health endpoint `GET /health` reporting connection counts by status
    - _Requirements: 1.1_

  - [x] 11.2 Implement session management
    - Create `baileys-service/src/session-manager.ts`
    - Manage one `WASocket` per active organization (keyed by realmId)
    - Persist auth state encrypted (AES-256-GCM, key from `SESSION_ENCRYPTION_KEY` env)
    - Connection state machine: `disconnected → qr_pending → connected` / `→ logged_out`
    - Enforce max concurrent connections (default 100 from `MAX_CONNECTIONS` env)
    - _Requirements: 1.1, 1.2, 1.5, 1.6, 1.7_

  - [x] 11.3 Write property test for session encryption round-trip
    - **Property 1: Session Encryption Round-Trip**
    - **Validates: Requirement 1.6**

  - [x] 11.4 Implement auto-reconnect logic
    - On recoverable disconnect (connection closed, timed out, lost): attempt reconnection with exponential backoff (2s start, 60s max, 5 attempts)
    - On remote logout: transition to `logged_out`, notify backend
    - On QR expiry (60s): regenerate up to 3 times, then transition to `disconnected`
    - _Requirements: 1.4, 1.5, 1.8_

  - [x] 11.5 Implement Baileys HTTP API endpoints
    - POST `/sessions/:realmId/start` — create WASocket, return QR data
    - POST `/sessions/:realmId/send` — send message to recipient phone
    - POST `/sessions/:realmId/stop` — disconnect and cleanup session
    - GET `/sessions/:realmId/status` — return current connection status
    - On incoming message: POST to backend's `/internal/whatsapp/incoming` with internal token
    - _Requirements: 1.1, 1.2, 1.3, 3.4_

  - [x] 11.6 Create Dockerfile for Baileys Service
    - Create `baileys-service/Dockerfile` — Node.js 20 Alpine image
    - Multi-stage build (install deps → copy source → run)
    - Add to existing `docker-compose.yml` as `baileys` service on private network
    - _Requirements: 10.3_

- [x] 12. Frontend configuration wizard
  - [x] 12.1 Create chatbot types and API service
    - Create `frontend/src/types/chatbot.rs` — Rust types mirroring backend DTOs
    - Create `frontend/src/services/chatbot.rs` — API calls for all chatbot endpoints (get_config, update_config, connect, disconnect, status, test_chat, list_conversations, pending_receipts, confirm_receipt, reject_receipt)
    - Register in respective `mod.rs` files
    - _Requirements: 9.1, 12.1_

  - [x] 12.2 Implement connection step component
    - Create `frontend/src/components/feature/chatbot/connection_step.rs`
    - QR code display (render base64 QR from API)
    - Connection status indicator (disconnected, qr_pending, connected, logged_out)
    - Connect/disconnect buttons
    - _Requirements: 1.1, 1.2, 1.3, 9.1_

  - [x] 12.3 Implement persona and capabilities step components
    - Create `frontend/src/components/feature/chatbot/persona_step.rs` — form for display_name, language, tone, greeting, system_prompt
    - Create `frontend/src/components/feature/chatbot/capabilities_step.rs` — toggle switches for each capability
    - Client-side validation matching backend constraints
    - _Requirements: 9.2, 9.3_

  - [x] 12.4 Implement knowledge and sender policy step components
    - Create `frontend/src/components/feature/chatbot/knowledge_step.rs` — FAQ editor (add/remove/edit pairs), policies textarea
    - Create `frontend/src/components/feature/chatbot/sender_policy_step.rs` — radio buttons for policy type + allowlist editor
    - _Requirements: 9.4, 9.5_

  - [x] 12.5 Implement test chat and activation step components
    - Create `frontend/src/components/feature/chatbot/test_chat_step.rs` — chat-style interface, send test messages, display AI responses
    - Create `frontend/src/components/feature/chatbot/activation_step.rs` — final toggle to activate chatbot
    - _Requirements: 12.1, 12.2, 12.3, 12.5_

  - [x] 12.6 Create chatbot configuration page with wizard layout
    - Create `frontend/src/pages/chatbot_config.rs` — wizard container with step navigation
    - Wire all step components into sequential flow
    - Add status dashboard (connection state, message counts, pending receipts)
    - Register page route at `/configuracion/chatbot`
    - _Requirements: 9.1, 8.4_

- [x] 13. Integration wiring and background jobs
  - [x] 13.1 Implement Baileys HTTP client in backend
    - Add `BaileysClient` to `backend/src/services/chatbot.rs` (or separate file)
    - Methods: `start_session(realm_id)`, `send_message(realm_id, phone, content)`, `stop_session(realm_id)`, `get_status(realm_id)`
    - Use `BAILEYS_SERVICE_URL` from config
    - _Requirements: 1.1, 3.4_

  - [x] 13.2 Add retention cleanup to background jobs
    - Extend `backend/src/services/background_jobs.rs` with daily chatbot conversation cleanup
    - For each org with chatbot config: delete conversations older than `retention_days`
    - _Requirements: 7.4_

  - [x] 13.3 Wire chatbot routes and module exports
    - Register all chatbot handlers in `backend/src/routes.rs`
    - Add `chatbot` module to `backend/src/handlers/mod.rs` and `backend/src/services/mod.rs`
    - Ensure startup validation: reject start if `BAILEYS_INTERNAL_TOKEN` is missing/empty or shorter than 32 chars
    - _Requirements: 10.4, 10.5_

  - [x] 13.4 Add rate limiting to chatbot endpoints
    - Apply existing `write_governor_conf` rate limiting to public chatbot endpoints
    - Apply rate limiting to internal webhook endpoint (per-org throttle)
    - _Requirements: 10.6_

- [x] 14. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- The Baileys Service (Node.js) is a separate project within the workspace — property tests for encryption (Property 1) use the Node.js test framework
- All Rust PBT files follow the `{domain}_pbt.rs` naming convention in `backend/src/services/`

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "11.1"] },
    { "id": 2, "tasks": ["1.3", "11.2", "11.4"] },
    { "id": 3, "tasks": ["2.1", "2.3", "2.6", "11.3", "11.5"] },
    { "id": 4, "tasks": ["2.2", "2.4", "2.5", "2.7", "2.8", "11.6"] },
    { "id": 5, "tasks": ["2.9", "2.10", "4.1", "4.7"] },
    { "id": 6, "tasks": ["4.2", "4.3", "4.8", "4.9"] },
    { "id": 7, "tasks": ["4.4", "4.5", "4.10"] },
    { "id": 8, "tasks": ["4.6", "6.1"] },
    { "id": 9, "tasks": ["6.2", "6.4"] },
    { "id": 10, "tasks": ["6.3", "6.5"] },
    { "id": 11, "tasks": ["6.6", "6.7"] },
    { "id": 12, "tasks": ["8.1", "9.1"] },
    { "id": 13, "tasks": ["8.2", "8.3", "9.2", "9.3"] },
    { "id": 14, "tasks": ["8.4", "8.5", "9.4", "9.5"] },
    { "id": 15, "tasks": ["8.6", "8.7", "12.1"] },
    { "id": 16, "tasks": ["12.2", "12.3", "12.4"] },
    { "id": 17, "tasks": ["12.5", "12.6"] },
    { "id": 18, "tasks": ["13.1", "13.2", "13.3"] },
    { "id": 19, "tasks": ["13.4"] }
  ]
}
```
