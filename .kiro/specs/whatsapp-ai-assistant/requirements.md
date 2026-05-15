# Requirements Document

## Introduction

This document defines the requirements for the WhatsApp AI Assistant integration — a per-organization chatbot that enables tenants to interact with the property management platform via WhatsApp. Tenants can query balances, submit payment receipts via photo, request maintenance, and get answers from landlord-configured FAQs. The system uses a Node.js sidecar (Baileys) for WhatsApp connectivity and a Rust AI module (Rig + OVMS) for conversational intelligence.

## Glossary

- **Chatbot_System**: The complete WhatsApp AI assistant integration including the Baileys sidecar, backend handlers, AI module, and conversation storage
- **AI_Module**: The Rig-based conversational agent that processes messages, invokes tools, and generates responses via OVMS
- **Baileys_Service**: The Node.js sidecar responsible for WhatsApp Web connectivity, QR pairing, session management, and message transport
- **Sender_Policy_Engine**: The component that determines whether an incoming sender is authorized to interact with the chatbot based on the organization's configured policy
- **Receipt_Extractor**: The AI tool that extracts structured payment data (bank, amount, currency, date, reference) from receipt images with confidence scoring
- **Conversation_Store**: The PostgreSQL-backed storage for message history with per-organization retention policies
- **Configuration_Wizard**: The frontend onboarding interface that guides landlords through chatbot setup (connection, persona, capabilities, knowledge, sender policy, testing, activation)
- **OVMS**: OpenVINO Model Server — the self-hosted LLM inference endpoint (OpenAI-compatible API)
- **Tenant_Context**: Resolved tenant information (name, contracts, balances) used to personalize AI responses
- **Confidence**: A three-level classification (High, Medium, Low) derived from OCR confidence scores for receipt extraction results

## Requirements

### Requirement 1: WhatsApp Connection Management

**User Story:** As a property manager (admin/gerente), I want to connect my organization's WhatsApp number to the chatbot, so that tenants can reach the system through a familiar messaging channel.

#### Acceptance Criteria

1. WHEN a property manager initiates a WhatsApp connection, THE Baileys_Service SHALL generate a QR code within 5 seconds and transition the connection status to `qr_pending`
2. WHEN a WhatsApp device scans the QR code successfully, THE Baileys_Service SHALL transition the connection status to `connected` and persist the authenticated session
3. WHEN a property manager requests disconnection, THE Baileys_Service SHALL terminate the WhatsApp session and transition the connection status to `disconnected`
4. IF the WhatsApp connection drops due to a recoverable error (connection closed, timed out, or connection lost — excluding explicit logout by the remote device), THEN THE Baileys_Service SHALL transition the connection status to `reconnecting`, attempt automatic reconnection using exponential backoff (starting at 2 seconds, maximum 60 seconds) for up to 5 attempts without requiring a new QR scan, and transition the connection status to `disconnected` if all attempts fail
5. IF the WhatsApp session is logged out remotely, THEN THE Baileys_Service SHALL transition the connection status to `logged_out` and notify the backend via the session status API
6. THE Baileys_Service SHALL persist session authentication state encrypted at rest using AES-256-GCM
7. IF a property manager initiates a WhatsApp connection and the number of active connections has reached the configured maximum (default 100), THEN THE Baileys_Service SHALL reject the connection request with an error indicating capacity is full
8. IF the QR code expires before being scanned (within 60 seconds of generation), THEN THE Baileys_Service SHALL automatically generate a new QR code up to 3 times, and transition the connection status to `disconnected` if all attempts expire without a successful scan

### Requirement 2: Sender Policy Enforcement

**User Story:** As a property manager, I want to control who can interact with my chatbot, so that only authorized senders consume AI resources and receive responses.

#### Acceptance Criteria

1. WHEN an incoming message arrives and the sender policy is `tenants_only`, THE Sender_Policy_Engine SHALL allow the message only if the sender phone matches a tenant within the same organization
2. WHEN an incoming message arrives and the sender policy is `tenants_and_prospects`, THE Sender_Policy_Engine SHALL allow messages from all senders
3. WHEN an incoming message arrives and the sender policy is `allowlist`, THE Sender_Policy_Engine SHALL allow the message only if the sender phone appears in the organization's configured allowlist
4. IF a sender is not authorized by the active policy, THEN THE Chatbot_System SHALL silently discard the message without invoking the AI_Module and without sending any reply to the sender, but SHALL log the discarded message for monitoring and analytics purposes
5. THE Sender_Policy_Engine SHALL resolve phone-to-tenant mappings scoped exclusively to the message's organization (no cross-organization resolution)
6. IF the organization's sender policy is not configured or contains an unrecognized value, THEN THE Sender_Policy_Engine SHALL deny the message by default (fail-closed)

### Requirement 3: AI Conversational Agent

**User Story:** As a tenant, I want to have a natural conversation with the chatbot in Spanish, so that I can get help with my property-related questions without calling the landlord.

#### Acceptance Criteria

1. WHEN a message (text, image, or text with image attachment) is received from an authorized sender, THE AI_Module SHALL compose a system prompt from the organization's persona configuration (tone, greeting, system prompt, language), FAQs, policies, and resolved tenant context
2. WHEN processing a message, THE AI_Module SHALL include the last N messages from conversation history (where N equals the organization's configured `history_limit`) as context for the LLM
3. THE AI_Module SHALL send chat completions to the self-hosted OVMS endpoint using the OpenAI-compatible API with a timeout of 30 seconds
4. WHEN the LLM generates a response and no error condition is detected, THE Chatbot_System SHALL deliver the reply to the sender via the Baileys_Service
5. THE AI_Module SHALL register tools on the agent only if the corresponding capability is enabled in the organization's chatbot configuration
6. WHEN a capability toggle is disabled, THE AI_Module SHALL immediately unregister the corresponding tool from the agent, making it unavailable to the LLM for the current and all subsequent messages
7. IF the OVMS endpoint is unreachable or the chat completion request exceeds the 30-second timeout, THEN THE AI_Module SHALL return an error message to the sender indicating the service is temporarily unavailable and persist the user's original message in conversation history

### Requirement 4: Receipt Image Extraction

**User Story:** As a tenant, I want to send a photo of my payment receipt via WhatsApp, so that my payment can be recorded without visiting the office.

#### Acceptance Criteria

1. WHEN the LLM determines an attached image is a payment receipt, THE Receipt_Extractor SHALL extract structured data including bank, amount (0.01 to 999,999,999.99), currency (DOP or USD), date, reference number (up to 50 characters), sender name, and recipient
2. WHEN extraction completes, THE Receipt_Extractor SHALL assign a confidence level: High (score ≥ 0.85), Medium (score 0.60–0.84), or Low (score < 0.60)
3. WHEN a receipt is extracted with High confidence and the tenant is resolved, THE Chatbot_System SHALL record the payment as a pending confirmation entry and reply to the tenant indicating the receipt was received and is awaiting landlord confirmation
4. WHEN a receipt is extracted with High confidence and the tenant cannot be resolved, THE Chatbot_System SHALL queue the extraction for landlord confirmation with status `pending_confirmation`
5. WHEN a receipt is extracted with Medium or Low confidence, THE Chatbot_System SHALL queue the extraction for landlord confirmation regardless of tenant resolution status and reply to the tenant indicating the receipt requires manual review
6. IF the Receipt_Extractor encounters a complete extraction failure (OCR service unavailable or processing timeout after 30 seconds), THEN THE Chatbot_System SHALL send a reply to the tenant indicating the image could not be processed and suggest resending a clearer photo
7. THE Receipt_Extractor SHALL store all extraction results in the `chatbot_receipt_extraction` table with the originating conversation reference

### Requirement 5: Balance Query

**User Story:** As a tenant, I want to ask the chatbot how much I owe, so that I can check my balance without logging into the platform.

#### Acceptance Criteria

1. WHEN the LLM invokes the `query_balance` tool for a resolved tenant, THE Chatbot_System SHALL return all payments with status `pendiente` or `atrasado`, each showing the payment amount, currency, and due date, plus a total outstanding balance grouped by currency
2. IF the LLM invokes the `query_balance` tool and the sender cannot be resolved to a tenant, THEN THE Chatbot_System SHALL return an error indicating the tenant could not be identified
3. THE Chatbot_System SHALL display monetary amounts with the currency symbol (RD$ for DOP, US$ for USD) and exactly two decimal places
4. IF a tenant has outstanding payments in both DOP and USD, THEN THE Chatbot_System SHALL present separate totals per currency (never sum amounts across different currencies)

### Requirement 6: Maintenance Request Creation

**User Story:** As a tenant, I want to report maintenance issues through WhatsApp, so that I can submit requests conveniently without using the web platform.

#### Acceptance Criteria

1. WHEN the LLM invokes the `create_maintenance_request` tool with a description (between 1 and 1000 characters) from a resolved tenant, THE Chatbot_System SHALL create a maintenance request (SolicitudMantenimiento) linked to the tenant's active contract property and respond to the tenant with a confirmation message indicating the request was created
2. IF the resolved tenant has multiple active contracts, THEN THE Chatbot_System SHALL link the maintenance request to the property identified by the LLM from conversation context, or to the most recently started contract's property if the LLM does not specify
3. WHEN a maintenance request is created via chatbot, THE Chatbot_System SHALL set the initial status to `pendiente` and priority to `media` unless the LLM specifies a valid priority value (`baja`, `media`, `alta`, or `urgente`) based on conversation context
4. IF the LLM invokes `create_maintenance_request` with a description that is empty, fewer than 2 characters, or exceeding 1000 characters, THEN THE Chatbot_System SHALL reject the request and respond to the sender indicating a valid description is required
5. IF the sender cannot be resolved to a tenant with an active contract, THEN THE Chatbot_System SHALL inform the sender that maintenance requests require an active tenancy

### Requirement 7: Conversation History Management

**User Story:** As a property manager, I want conversation history to be stored and managed automatically, so that the chatbot has context for ongoing conversations and old data is cleaned up.

#### Acceptance Criteria

1. WHEN a message exchange occurs (user message and assistant reply), THE Conversation_Store SHALL persist both messages with the sender phone, organization, role, content, message type, and timestamp
2. WHEN loading conversation context for the AI_Module, THE Conversation_Store SHALL retrieve messages for the specific (organizacion_id, sender_phone) pair, ordered by creation time descending, limited to the organization's configured `history_limit` (default 10, range 1–50)
3. THE Conversation_Store SHALL index conversations by `(organizacion_id, sender_phone, created_at DESC)` for efficient history retrieval
4. WHEN a conversation record's age exceeds the organization's configured `retention_days` (default 90), THE Chatbot_System SHALL delete the record during the daily retention cleanup job
5. WHEN a sender phone can be resolved to a tenant, THE Conversation_Store SHALL record the `inquilino_id` on the conversation message at the time of persistence
6. IF the AI_Module fails to generate a reply after receiving a user message, THEN THE Conversation_Store SHALL still persist the user message, and THE Chatbot_System SHALL deliver an error message to the sender indicating the system could not process the request

### Requirement 8: Receipt Confirmation Workflow

**User Story:** As a property manager, I want to review and confirm or reject receipt extractions, so that only valid payments are recorded in the system.

#### Acceptance Criteria

1. WHEN a receipt extraction is pending confirmation, THE Chatbot_System SHALL make it available in the pending receipts list for the organization's admin or gerente, ordered by creation date descending
2. WHEN a property manager confirms a receipt extraction, THE Chatbot_System SHALL update the extraction status to `confirmed`, record the confirming user, and create a Pago record with status `pagado` linked to the extraction's matched contract using the extracted amount, currency, and date
3. WHEN a property manager rejects a receipt extraction, THE Chatbot_System SHALL update the extraction status to `rejected` and persist an optional rejection reason provided by the property manager
4. THE Chatbot_System SHALL display extracted receipt data (bank, amount, currency, date, reference, confidence) along with the associated tenant name and contract reference to the property manager for review
5. IF a property manager attempts to confirm or reject a receipt extraction that is not in `pending_confirmation` status, THEN THE Chatbot_System SHALL reject the action with an error indicating the extraction has already been processed
6. THE Chatbot_System SHALL enforce atomic operations on receipt extraction status transitions, preventing concurrent conflicting actions (such as simultaneous confirm and reject) on the same extraction

### Requirement 9: Chatbot Configuration

**User Story:** As a property manager, I want to configure the chatbot's personality, capabilities, and knowledge base, so that it represents my organization appropriately.

#### Acceptance Criteria

1. THE Configuration_Wizard SHALL provide a step-by-step onboarding flow covering: connection, persona, capabilities, knowledge, sender policy, testing, and activation
2. WHEN a property manager updates the persona settings, THE Chatbot_System SHALL validate the display name (1–100 characters), language, tone (1–50 characters), greeting, and system prompt override, and SHALL persist the settings only after all validations pass successfully
3. WHEN a property manager toggles capability switches, THE Chatbot_System SHALL persist the enabled/disabled state for each capability (receipt_ocr, balance_queries, payment_reminders, maintenance_requests, human_handoff)
4. WHEN a property manager edits FAQs, THE Chatbot_System SHALL persist the FAQ entries as structured question-answer pairs, enforcing a maximum of 50 entries with each question limited to 200 characters and each answer limited to 1000 characters
5. WHEN a property manager updates policies text, THE Chatbot_System SHALL persist the free-form policies content (maximum 5000 characters) for inclusion in the AI system prompt
6. THE Chatbot_System SHALL require the `admin` or `gerente` role for all chatbot configuration operations
7. WHEN the chatbot `activo` flag is set to false, THE Chatbot_System SHALL discard incoming messages for that organization without sending any reply to the sender
8. WHEN a property manager saves any configuration change, THE Chatbot_System SHALL apply the updated configuration starting from the next incoming message processed for that organization
9. IF a property manager submits persona settings with a display name that is empty or exceeds 100 characters, THEN THE Chatbot_System SHALL reject the update with an error message indicating the invalid field

### Requirement 10: Internal Webhook Security

**User Story:** As a system architect, I want the internal webhook between Baileys and the backend to be authenticated, so that only the legitimate sidecar can submit incoming messages.

#### Acceptance Criteria

1. WHEN the Baileys_Service sends an incoming message to the backend webhook, THE Chatbot_System SHALL require a valid `X-Internal-Token` header whose value matches the configured shared secret using a constant-time comparison
2. IF an incoming webhook request has a missing or invalid `X-Internal-Token` header, THEN THE Chatbot_System SHALL reject the request with HTTP 401 status, not process the message, and log the rejection (without logging the token value)
3. THE Chatbot_System SHALL rely on deployment configuration to ensure the webhook endpoint is not publicly accessible; runtime network-type validation is not required
4. IF the `BAILEYS_INTERNAL_TOKEN` environment variable is not configured or is empty at startup, THEN THE Chatbot_System SHALL refuse to start and report a configuration error
5. THE Chatbot_System SHALL require the configured shared secret to be at least 32 characters in length
6. THE Chatbot_System SHALL allow rejection of webhook requests for reasons beyond invalid tokens, including rate limiting, maintenance mode, or malformed message content

### Requirement 11: Human Handoff

**User Story:** As a tenant, I want to request to speak with a human when the chatbot cannot help me, so that I can escalate complex issues.

#### Acceptance Criteria

1. WHEN the LLM invokes the `handoff_to_human` tool, THE Chatbot_System SHALL set the conversation's handoff status to `awaiting_human` and persist this state in the conversation record for the sender phone and organization
2. WHEN the LLM invokes the `handoff_to_human` tool, THE Chatbot_System SHALL deliver a final AI-generated acknowledgment message to the sender indicating that a human operator has been notified
3. WHEN a property manager configures handoff keywords, THE Chatbot_System SHALL include those keywords in the AI system prompt so the LLM can use them as context when deciding whether to invoke the `handoff_to_human` tool
4. WHILE a conversation has handoff status `awaiting_human`, THE Chatbot_System SHALL not invoke the AI_Module for incoming messages from that sender, and SHALL persist those messages in the Conversation_Store for human operator review
5. WHEN an admin or gerente clears the handoff status for a conversation, THE Chatbot_System SHALL set the handoff status back to `none` and resume AI processing for subsequent messages from that sender
6. IF the `human_handoff` capability is disabled in the organization's configuration, THEN THE Chatbot_System SHALL not register the `handoff_to_human` tool on the AI agent

### Requirement 12: Test Chat

**User Story:** As a property manager, I want to test the chatbot within the configuration interface, so that I can verify its behavior before activating it for tenants.

#### Acceptance Criteria

1. WHEN a property manager sends a test message through the configuration UI, THE Chatbot_System SHALL process it through the full AI pipeline (system prompt, tools, history) and return the response in the UI without sending any message through the Baileys_Service
2. THE Configuration_Wizard SHALL provide a chat-style interface for sending test messages and viewing AI responses
3. WHEN testing, THE Chatbot_System SHALL use the organization's current (possibly unsaved draft) configuration for the AI persona and capabilities
4. THE Chatbot_System SHALL NOT persist test chat messages in the `chatbot_conversation` table used for real tenant conversations, but MAY store them in a separate debugging or audit table
5. IF the AI pipeline fails, times out, or encounters internal issues (such as degraded model performance or partial tool failures) during a test message, THEN THE Chatbot_System SHALL return an error indication to the configuration UI describing the failure reason

### Requirement 13: LLM Inference Infrastructure (OVMS)

**User Story:** As a system architect, I want the LLM inference to run on a self-hosted OpenVINO Model Server instance on the existing Intel GPU node, so that the chatbot has low-latency access to a capable language model without external API dependencies.

#### Acceptance Criteria

1. THE system SHALL deploy an OpenVINO Model Server (OVMS) instance as a Kubernetes Deployment in the `realestate` namespace, serving the Qwen3-30B-A3B MoE model (30B total parameters, 3B active per token) quantized to INT4 precision via the OpenVINO IR format
2. THE OVMS instance SHALL expose an OpenAI-compatible API on port 8000, supporting `/v3/chat/completions` and `/v3/completions` endpoints with streaming and non-streaming modes
3. THE OVMS instance SHALL be scheduled on the `coreos` node (the control plane node with Intel GPU) using a node selector matching `kubernetes.io/hostname: coreos`
4. THE OVMS instance SHALL request `gpu.intel.com/i915: "1"` resource to access the Intel GPU for inference acceleration
5. THE OVMS instance SHALL use continuous batching with paged attention for efficient concurrent request handling
6. THE OVMS instance SHALL be configured with a KV cache size appropriate for the available GPU memory, accounting for shared GPU usage with the OCR service (PaddleOCR)
7. IF the Intel discrete GPU cannot accommodate both the LLM and PaddleOCR simultaneously, THEN the OCR service MAY be reconfigured to use the integrated GPU (iGPU) of the control plane node instead, and the discrete GPU SHALL be dedicated to OVMS
8. THE OVMS deployment SHALL include a readiness probe on the `/v1/config` endpoint to verify the model is loaded and available before accepting traffic
9. THE OVMS deployment SHALL include a liveness probe to detect and recover from inference engine failures
10. THE model preparation step SHALL convert the Qwen3-30B-A3B model to OpenVINO IR format with INT4 weight compression and GPU-optimized KV cache configuration, and bake the converted model into the container image or mount it via a persistent volume
11. THE backend SHALL connect to OVMS using the `OVMS_ENDPOINT` environment variable (default `http://ovms:8000/v3`) and `OVMS_CHAT_MODEL` (default `Qwen/Qwen3-30B-A3B`)
12. THE OVMS instance SHALL have telemetry disabled (`OPENVINO_TELEMETRY_ENABLE=0`) and run as a non-root user with a read-only root filesystem
13. THE system SHALL expose a Kubernetes Service named `ovms` on port 8000 within the `realestate` namespace for internal backend access

### Requirement 14: Phone Number Validation and Format

**User Story:** As a system operator, I want all phone numbers stored and processed in a consistent format, so that tenant resolution and message routing work reliably.

#### Acceptance Criteria

1. THE Chatbot_System SHALL store all phone numbers in E.164 format, defined as a `+` character followed by 1 to 15 digits where the first digit is not zero (matching the pattern `^\+[1-9]\d{1,14}$`)
2. WHEN resolving a sender phone to a tenant, THE Chatbot_System SHALL match against the `telefono` field in the `inquilinos` table scoped to the message's organization, and return no match if zero records are found
3. IF a phone number does not conform to E.164 format on input, THEN THE Chatbot_System SHALL attempt normalization by stripping non-digit characters, removing leading trunk zeros, and prepending the configured default country code prefix before processing
4. IF a phone number cannot be normalized to a valid E.164 number after normalization attempts (fewer than 7 digits after stripping or contains non-numeric characters other than a leading `+`), THEN THE Chatbot_System SHALL reject the number with a validation error and not process the message
