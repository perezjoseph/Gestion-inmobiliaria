# Technical Design: WhatsApp AI Assistant Integration

## Overview

Adds a per-organization WhatsApp chatbot that lets tenants query balances, submit payment receipts via photo, request maintenance, and get answers from landlord-configured FAQs — all through WhatsApp. The system uses a Node.js sidecar (Baileys) for WhatsApp connectivity and a Rust AI module (Rig + OVMS) for conversational intelligence and receipt extraction.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (Leptos)                         │
│  /configuracion/chatbot → onboarding wizard, status, test chat  │
└────────────────────────────────┬────────────────────────────────┘
                                 │ REST
┌────────────────────────────────▼────────────────────────────────┐
│                     Backend (Actix-web)                          │
│                                                                  │
│  handlers/chatbot.rs ─── services/chatbot.rs                    │
│  handlers/chatbot_internal.rs (incoming webhook)                │
│                                                                  │
│  ┌──────────────────┐   ┌──────────────────┐                   │
│  │  AI Module (Rig) │   │  Conversation    │                   │
│  │  - chat flow     │   │  History Store   │                   │
│  │  - receipt OCR   │   │  (PostgreSQL)    │                   │
│  └────────┬─────────┘   └──────────────────┘                   │
│           │                                                      │
└───────────┼──────────────────────────────────────────────────────┘
            │ OpenAI-compat API                    │ HTTP
            ▼                                      ▼
┌───────────────────┐              ┌───────────────────────────────┐
│   OVMS Instance   │              │     Baileys Service (Node)    │
│  (LLM + Vision)   │              │  - WS connections per org     │
│                    │              │  - QR generation              │
└───────────────────┘              │  - message send/receive       │
                                   └───────────────────────────────┘
```

## Data Model

### New Entities

#### `chatbot_config` (one per organization)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| organizacion_id | UUID FK UNIQUE | one-to-one with organizaciones |
| activo | BOOLEAN | master on/off, default false |
| connection_status | VARCHAR(20) | `disconnected`, `qr_pending`, `connected`, `logged_out` |
| display_name | VARCHAR(100) | bot display name |
| language | VARCHAR(10) | default `es-DO` |
| tone | VARCHAR(50) | e.g. `profesional`, `amigable` |
| greeting | TEXT | first-contact message |
| system_prompt | TEXT | custom system prompt override |
| faqs | JSONB | `[{question, answer}]` |
| policies | TEXT | free-form policies text |
| sender_policy | VARCHAR(30) | `tenants_only`, `tenants_and_prospects`, `allowlist` |
| allowlist | JSONB | `["18091234567", ...]` |
| capabilities | JSONB | `{receipt_ocr, balance_queries, payment_reminders, maintenance_requests, human_handoff}` |
| handoff_keywords | JSONB | `["hablar con humano", "agente"]` |
| history_limit | INTEGER | messages per context window, default 10 |
| retention_days | INTEGER | conversation history TTL, default 90 |
| updated_by | UUID FK | last modifier |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

#### `chatbot_conversation` (message history)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| organizacion_id | UUID FK | scoping |
| sender_phone | VARCHAR(20) | E.164 format |
| inquilino_id | UUID FK NULL | resolved tenant, if matched |
| role | VARCHAR(10) | `user` or `assistant` |
| content | TEXT | message body |
| message_type | VARCHAR(20) | `text`, `image`, `receipt_result` |
| metadata | JSONB NULL | extracted receipt data, confidence, etc. |
| created_at | TIMESTAMPTZ | |

**Indexes**: `(organizacion_id, sender_phone, created_at DESC)` for history retrieval, `(organizacion_id, created_at)` for retention cleanup.

#### `chatbot_receipt_extraction` (pending receipt confirmations)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| organizacion_id | UUID FK | |
| conversation_id | UUID FK | originating message |
| inquilino_id | UUID FK NULL | matched tenant |
| contrato_id | UUID FK NULL | matched contract |
| extracted_data | JSONB | `{bank, amount, date, reference, sender_name, recipient, confidence}` |
| status | VARCHAR(20) | `pending_confirmation`, `confirmed`, `rejected` |
| confirmed_by | UUID FK NULL | landlord who confirmed |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

### Migration

File: `m20260509_000001_create_chatbot_tables.rs`

## API Design

### Public Endpoints (landlord-facing)

All under `/api/v1/chatbot`, require auth + `admin` or `gerente` role.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/config` | Get chatbot config for current org |
| PUT | `/config` | Create or update chatbot config |
| POST | `/connect` | Initiate WhatsApp connection (returns QR or status) |
| POST | `/disconnect` | Disconnect and clear session |
| GET | `/status` | Get current connection status |
| GET | `/conversations` | List recent conversations (paginated) |
| GET | `/conversations/{phone}` | Get conversation history with a phone |
| GET | `/receipts/pending` | List pending receipt confirmations |
| POST | `/receipts/{id}/confirm` | Confirm a receipt extraction |
| POST | `/receipts/{id}/reject` | Reject a receipt extraction |
| POST | `/test` | Send a test message through the AI (in-UI testing) |

### Internal Endpoint (Baileys → Backend)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/internal/whatsapp/incoming` | Webhook for incoming messages |

Authenticated via `X-Internal-Token` header (shared secret, private network only).

**Request body:**
```json
{
  "realmId": "uuid",
  "senderPhone": "+18091234567",
  "messageType": "text|image",
  "content": "text content or base64 image",
  "caption": "optional image caption",
  "messageId": "whatsapp-msg-id",
  "timestamp": 1715200000
}
```

### Baileys Service API (Backend → Baileys)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/sessions/{realmId}/start` | Start connection, returns QR data |
| POST | `/sessions/{realmId}/send` | Send message to recipient |
| POST | `/sessions/{realmId}/stop` | Disconnect session |
| GET | `/sessions/{realmId}/status` | Get connection status |
| GET | `/health` | Service health check |

## Component Design

### 1. Baileys Service (Node.js sidecar)

**Location**: `baileys-service/`

- Express.js HTTP server
- One Baileys `WASocket` per active organization
- Auth state persisted to `./sessions/{realmId}/` on disk (encrypted at rest via AES-256, key from env)
- On incoming message: POST to backend's internal webhook
- Connection state machine: `disconnected → qr_pending → connected` / `→ logged_out`
- Auto-reconnect on recoverable disconnects (Baileys `DisconnectReason`)
- Max 100 concurrent connections (configurable via env)
- Health endpoint reports connection counts by status

### 2. Backend Chatbot Domain

**Files** (following project conventions):

- `entities/chatbot_config.rs` — SeaORM entity
- `entities/chatbot_conversation.rs` — SeaORM entity
- `entities/chatbot_receipt_extraction.rs` — SeaORM entity
- `models/chatbot.rs` — DTOs (request/response)
- `services/chatbot.rs` — business logic (config CRUD, message routing, receipt flow)
- `handlers/chatbot.rs` — public API handlers
- `handlers/chatbot_internal.rs` — internal webhook handler
- `services/ai_module.rs` — Rig integration (chat + extraction)

### 3. AI Module (Rig) — Tool-Use Agent

**Responsibilities:**
- Compose system prompts from persona config + tenant context + FAQs + policies
- Manage conversation history window (last N messages)
- Chat completion via OVMS (OpenAI-compatible endpoint)
- Expose **tools** the LLM can invoke autonomously during conversation

**Stateless design**: receives all context as function parameters, no internal state.

**Agent with tools pattern**: The LLM is configured as a Rig agent with registered tools. It decides when to call them based on conversation context — the message router does NOT pre-classify messages.

```rust
pub struct AiModule {
    client: rig::providers::openai::Client, // pointed at OVMS
}

impl AiModule {
    /// Single entry point for all messages (text or image).
    /// The LLM decides which tools to invoke based on content.
    pub async fn process_message(
        &self,
        config: &ChatbotPersona,
        tenant_context: Option<&TenantContext>,
        history: &[ConversationMessage],
        user_message: &UserMessage, // text + optional image attachment
    ) -> Result<AgentResponse, AppError>;
}
```

**Registered Tools:**

| Tool | Description | When LLM invokes it |
|------|-------------|---------------------|
| `extract_receipt` | Runs OCR + structured extraction on an attached image | User sends a payment receipt photo |
| `query_balance` | Looks up tenant's pending payments and balance | User asks about what they owe |
| `create_maintenance_request` | Creates a maintenance ticket | User reports a problem |
| `get_payment_history` | Returns recent payment records | User asks about past payments |
| `handoff_to_human` | Flags conversation for human operator | User requests human help or matches handoff keywords |

Each tool is a Rig `Tool` impl that receives typed input from the LLM and calls the appropriate backend service.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractReceiptInput {
    pub image_base64: String,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub bank: Option<String>,
    pub amount: Decimal,
    pub currency: String,       // "DOP" or "USD"
    pub date: Option<String>,   // ISO 8601
    pub reference: Option<String>,
    pub sender_name: Option<String>,
    pub recipient: Option<String>,
    pub confidence: Confidence, // High, Medium, Low
}
```

**Tool capability gating**: Tools are only registered on the agent if the corresponding capability toggle is enabled in `ChatbotConfig.capabilities`. If `receipt_ocr` is disabled, the `extract_receipt` tool is simply not available to the LLM.

### 4. Message Processing Pipeline

```
Incoming WhatsApp Message
    │
    ▼
[Internal Webhook Handler]
    │
    ├─ Validate internal token
    ├─ Load ChatbotConfig for org
    ├─ Check config.activo == true
    ├─ Apply sender policy filter
    │
    ▼
[AI Agent Invocation]
    │
    ├─ Resolve phone → tenant (if possible)
    ├─ Load conversation history (last N)
    ├─ Compose system prompt (persona + FAQs + policies + tenant context)
    ├─ Build agent with enabled tools only (based on capabilities config)
    ├─ Pass message (text + optional image attachment) to agent
    │
    ▼
[LLM decides autonomously]
    │
    ├─ Plain conversation → generates text reply
    ├─ Image received → may call extract_receipt tool
    ├─ Balance question → calls query_balance tool
    ├─ Maintenance report → calls create_maintenance_request tool
    ├─ Handoff trigger → calls handoff_to_human tool
    │
    ▼
[Post-processing]
    │
    ├─ Persist user message + assistant reply in conversation history
    ├─ If receipt extracted with high confidence → record pending payment
    ├─ If receipt extracted with medium/low confidence → queue for landlord confirmation
    ├─ Send final reply via Baileys Service
```

The LLM is the router. The backend does not pre-classify message types — it passes everything to the agent and lets the model decide which tools to invoke based on context.

### 5. Sender Policy Resolution

```rust
fn is_sender_allowed(config: &ChatbotConfig, phone: &str, org_id: Uuid, db: &DbConn) -> bool {
    match config.sender_policy.as_str() {
        "tenants_only" => tenant_exists_by_phone(phone, org_id, db).await,
        "tenants_and_prospects" => true, // all senders allowed
        "allowlist" => config.allowlist.contains(phone),
        _ => false,
    }
}
```

Phone-to-tenant resolution: query `inquilinos` where `telefono = phone AND organizacion_id = org_id`. Never cross-org resolution.

### 6. Conversation History Management

- Store all messages in `chatbot_conversation`
- On each chat invocation, load last `config.history_limit` messages for `(org_id, sender_phone)`
- Background job: delete conversations older than `config.retention_days`
- Retention cleanup runs daily via existing background task infrastructure

### 7. Configuration & Environment

New env vars:
```
BAILEYS_SERVICE_URL=http://baileys:3100
BAILEYS_INTERNAL_TOKEN=<shared-secret>
OVMS_ENDPOINT=http://ovms:8000/v1
OVMS_CHAT_MODEL=qwen3.6
AI_CHAT_TIMEOUT_SECS=30
```

### 8. Frontend Pages

**Location**: `frontend/src/pages/chatbot_config.rs`

Onboarding wizard with steps:
1. **Connect** — QR code display, status indicator
2. **Persona** — name, language, tone, greeting, system prompt
3. **Capabilities** — toggle switches for each feature
4. **Knowledge** — FAQ editor, policies textarea
5. **Sender Policy** — radio buttons + allowlist editor
6. **Test** — in-UI chat simulator
7. **Activate** — final toggle

Status dashboard showing: connection state, message counts, pending receipts.

## Security Considerations

- Internal webhook authenticated via shared token over private network (no public exposure)
- Baileys auth state encrypted at rest (AES-256-GCM)
- Phone numbers stored in E.164 format, validated on input
- Sender policy enforced before any AI processing (no resource waste on unauthorized senders)
- Rate limiting on public chatbot endpoints (reuse existing `write_governor_conf`)
- No PII in logs — log org_id and phone hash only
- OVMS runs on private network, no public exposure

## Dependencies

### Backend (new Cargo deps)
- `rig-core` — AI framework for Rust
- No new HTTP client needed (already has `reqwest`)

### Baileys Service (new package)
- `@whiskeysockets/baileys` — WhatsApp Web client
- `express` — HTTP server
- `qrcode` — QR generation
- `pino` — structured logging

### Infrastructure
- OVMS container (already exists for OCR, extend with chat models)
- Baileys Service container (new)

## Out of Scope (V1)

- Multi-process Baileys sharding
- WhatsApp Business Cloud API migration
- Voice message transcription
- Group chat participation
- Sophisticated retry/backoff for outbound messages
- In-thread human↔AI handoff (handoff drops AI entirely)
- Multi-language switching within a conversation

## Open Questions

1. **Qwen3.6 variant**: Two open-weight options — Qwen3.6-27B (dense) or Qwen3.6-35B-A3B (MoE, 3B active per token). The MoE variant is more memory-efficient for inference. Confirm which fits on the Intel GPU alongside PaddleOCR, and whether OVMS supports the MoE architecture or if an alternative serving layer (vLLM, llama.cpp) is needed.
2. **Vision model for receipt OCR**: Qwen3.6 is text-only. For receipt image extraction, options: (a) pipe images through the existing OCR service first, then pass extracted text to Qwen3.6 as a tool result, or (b) add a separate vision model (Qwen3-VL). Option (a) reuses existing infrastructure.
3. **Receipt confidence thresholds**: The existing OCR returns raw float confidence (0.0–1.0) per text line. The chatbot needs High/Medium/Low for the `PaymentReceipt`. Proposed mapping: ≥0.85 = High, 0.60–0.84 = Medium, <0.60 = Low. Needs calibration with real Dominican bank receipts.

## Resolved Questions

- **Baileys persistence**: Database-backed. The project uses PostgreSQL for all state and Docker volumes only for binary files. Storing Baileys auth credentials in a `chatbot_sessions` table (encrypted JSONB column) is consistent with existing patterns and survives container recreation without volume mounts.

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Session Encryption Round-Trip

*For any* session authentication state data, encrypting with AES-256-GCM then decrypting with the same key SHALL produce the original data, and the ciphertext SHALL differ from the plaintext.

**Validates: Requirement 1.6**

### Property 2: Sender Policy Correctness

*For any* incoming message with sender phone P and organization O:
- If sender_policy is `tenants_only`, the message is allowed if and only if P exists as a tenant in O
- If sender_policy is `tenants_and_prospects`, the message is always allowed
- If sender_policy is `allowlist`, the message is allowed if and only if P appears in O's configured allowlist

**Validates: Requirements 2.1, 2.2, 2.3**

### Property 3: Organization-Scoped Tenant Resolution Isolation

*For any* phone number P that exists as a tenant in organization A but not in organization B, resolving P in the context of organization B SHALL return no match.

**Validates: Requirements 2.5, 13.2**

### Property 4: Unauthorized Senders Do Not Invoke AI

*For any* incoming message from a sender not authorized by the active sender policy, or for any organization with `activo` set to false, the AI_Module SHALL not be invoked and no AI resources are consumed.

**Validates: Requirements 2.4, 9.7**

### Property 5: System Prompt Composition Completeness

*For any* chatbot persona configuration (tone, greeting, system_prompt, FAQs, policies) and optional tenant context, the composed system prompt SHALL contain: the configured tone keyword, the greeting text, every FAQ question-answer pair, the policies text, and the tenant's name (when resolved).

**Validates: Requirement 3.1**

### Property 6: Conversation History Windowing

*For any* conversation history of length M and a configured history_limit of N, loading context for the AI_Module SHALL return exactly min(M, N) messages, consisting of the most recent messages ordered by creation time.

**Validates: Requirements 3.2, 7.2**

### Property 7: Tool Registration Matches Capabilities

*For any* chatbot capabilities configuration (a subset of {receipt_ocr, balance_queries, payment_reminders, maintenance_requests, human_handoff}), the set of tools registered on the AI agent SHALL equal exactly the set of tools corresponding to enabled capabilities — no more, no fewer.

**Validates: Requirements 3.5, 3.6**

### Property 8: Confidence Level Mapping

*For any* OCR confidence score S in [0.0, 1.0]:
- S ≥ 0.85 maps to High
- 0.60 ≤ S < 0.85 maps to Medium
- S < 0.60 maps to Low

The mapping SHALL be exhaustive (every valid score maps to exactly one level) and monotonic (higher scores never map to lower confidence levels).

**Validates: Requirement 4.2**

### Property 9: Confidence-Based Receipt Routing

*For any* receipt extraction result: if confidence is High and the tenant is resolved, the extraction SHALL be recorded as `pending_confirmation`. If confidence is Medium or Low, the extraction SHALL be queued for landlord confirmation regardless of tenant resolution status.

**Validates: Requirements 4.3, 4.4**

### Property 10: Balance Calculation Correctness

*For any* tenant with a set of payments where each payment has a status and amount, the `query_balance` tool SHALL return the sum of amounts for all payments with status `pendiente` or `atrasado` as the outstanding balance.

**Validates: Requirement 5.1**

### Property 11: Currency Formatting

*For any* monetary amount (Decimal) and currency (DOP or USD), the formatted display string SHALL contain the correct currency symbol (RD$ for DOP, US$ for USD) and exactly two decimal places.

**Validates: Requirement 5.3**

### Property 12: Maintenance Request Defaults and Linking

*For any* maintenance request created via the chatbot for a resolved tenant with an active contract, the request SHALL be linked to the contract's property (`propiedad_id`), have status `pendiente`, and have priority `media`.

**Validates: Requirements 6.1, 6.2**

### Property 13: Conversation Persistence Completeness

*For any* message exchange (user message and assistant reply), both persisted conversation records SHALL have non-null values for: `sender_phone`, `organizacion_id`, `role`, `content`, `message_type`, and `created_at`.

**Validates: Requirement 7.1**

### Property 14: Retention Cleanup Correctness

*For any* conversation message with creation timestamp T and organization retention_days R: if (now - T) > R days, the message SHALL be deleted by the cleanup job. If (now - T) ≤ R days, the message SHALL be preserved.

**Validates: Requirement 7.4**

### Property 15: Pending Receipts Visibility

*For any* receipt extraction with status `pending_confirmation` in organization O, querying the pending receipts list for O SHALL include that extraction in the results.

**Validates: Requirement 8.1**

### Property 16: Receipt Status Transitions

*For any* receipt extraction with status `pending_confirmation`: confirming it SHALL set status to `confirmed` and record the confirming user's ID. Rejecting it SHALL set status to `rejected`. No other status transitions are valid from `pending_confirmation`.

**Validates: Requirements 8.2, 8.3**

### Property 17: Configuration Round-Trip

*For any* valid chatbot configuration (persona settings, capability toggles, FAQ entries, policies text), saving the configuration then loading it SHALL produce values identical to the original input.

**Validates: Requirements 9.2, 9.3, 9.4, 9.5**

### Property 18: Role-Based Access Control for Configuration

*For any* user with a role other than `admin` or `gerente`, all chatbot configuration endpoints SHALL return a forbidden response.

**Validates: Requirement 9.6**

### Property 19: Internal Webhook Authentication

*For any* incoming webhook request: if the `X-Internal-Token` header is missing or does not match the configured shared secret, the request SHALL be rejected with an unauthorized response. If the token is valid, the request SHALL be accepted for processing.

**Validates: Requirements 10.1, 10.2**

### Property 20: Handoff Ceases AI Responses

*For any* conversation that has been flagged for human handoff, subsequent incoming messages from the same sender SHALL not invoke the AI_Module until the handoff is manually cleared.

**Validates: Requirement 11.3**

### Property 21: Phone Number E.164 Validation

*For any* input string: if it conforms to E.164 format (matches `^\+[1-9]\d{1,14}$`), it SHALL be accepted as-is. If it does not conform, the system SHALL either reject it with a validation error or normalize it to valid E.164 format before storage.

**Validates: Requirements 13.1, 13.3**
