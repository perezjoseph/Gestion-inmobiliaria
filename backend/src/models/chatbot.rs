use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- Confidence Level ---

/// Three-level confidence classification derived from OCR confidence scores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Maps a raw OCR confidence score (0.0–1.0) to a `Confidence` level.
///
/// Thresholds:
/// - score >= 0.85 → High
/// - score >= 0.60 → Medium
/// - score < 0.60 → Low
pub fn map_confidence(score: f64) -> Confidence {
    if score >= 0.85 {
        Confidence::High
    } else if score >= 0.60 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

// --- Configuration DTOs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaqEntry {
    /// Max 200 characters
    pub question: String,
    /// Max 1000 characters
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub receipt_ocr: bool,
    pub balance_queries: bool,
    pub payment_reminders: bool,
    pub maintenance_requests: bool,
    pub human_handoff: bool,
}

impl Capabilities {
    /// Returns a `Capabilities` instance with all flags enabled.
    /// Used for `AllWithHookGating` strategy where all tools are registered.
    pub fn all() -> Self {
        Self {
            receipt_ocr: true,
            balance_queries: true,
            payment_reminders: true,
            maintenance_requests: true,
            human_handoff: true,
        }
    }
}

// --- Agent Configuration ---

/// Per-organization agent configuration. Stored in chatbot_config.agent_config JSONB.
/// All fields are optional — defaults are applied when absent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Maximum multi-turn depth (tool call rounds). Default: 5, range: 1–15.
    pub max_turns: Option<u8>,
    /// LLM sampling temperature. Default: None (model default), range: 0.0–2.0.
    pub temperature: Option<f64>,
    /// Maximum response tokens. Default: None (model default), range: 1–4096.
    pub max_tokens: Option<u64>,
    /// Tool registration strategy.
    pub tool_registration: Option<ToolRegistrationStrategy>,
    /// Guardrail configuration overrides.
    pub guardrails: Option<GuardrailOverrides>,
}

impl AgentConfig {
    /// Resolves all optional fields to concrete values using defaults and clamping.
    pub fn resolve(&self) -> ResolvedAgentConfig {
        ResolvedAgentConfig {
            max_turns: self.max_turns.unwrap_or(5).clamp(1, 15) as usize,
            temperature: self.temperature.filter(|&t| (0.0..=2.0).contains(&t)),
            max_tokens: self.max_tokens.filter(|&t| (1..=4096).contains(&t)),
            tool_registration: self
                .tool_registration
                .clone()
                .unwrap_or(ToolRegistrationStrategy::Selective),
            guardrails: self.guardrails.clone().unwrap_or_default(),
        }
    }
}

/// How tools are registered on the agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolRegistrationStrategy {
    /// Only register tools for enabled capabilities (default, native Rig way).
    Selective,
    /// Register all tools but gate via hook (defense-in-depth, wastes tokens).
    AllWithHookGating,
}

/// Overridable guardrail thresholds per organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailOverrides {
    /// Max receipt amount in DOP before requiring human confirmation.
    pub max_receipt_amount_dop: Option<f64>,
    /// Max receipt amount in USD before requiring human confirmation.
    pub max_receipt_amount_usd: Option<f64>,
    /// Blocked output regex patterns (org-specific additions).
    pub blocked_patterns: Option<Vec<String>>,
    /// Whether to enable output safety filtering. Default: true.
    pub output_safety_enabled: Option<bool>,
}

impl Default for GuardrailOverrides {
    fn default() -> Self {
        Self {
            max_receipt_amount_dop: None,
            max_receipt_amount_usd: None,
            blocked_patterns: None,
            output_safety_enabled: Some(true),
        }
    }
}

/// Resolved agent configuration with all defaults applied and values clamped.
#[derive(Debug, Clone)]
pub struct ResolvedAgentConfig {
    /// Multi-turn depth, clamped to 1–15.
    pub max_turns: usize,
    /// Temperature if within valid range (0.0–2.0), None otherwise.
    pub temperature: Option<f64>,
    /// Max tokens if within valid range (1–4096), None otherwise.
    pub max_tokens: Option<u64>,
    /// Tool registration strategy (defaults to Selective).
    pub tool_registration: ToolRegistrationStrategy,
    /// Guardrail overrides (defaults applied).
    pub guardrails: GuardrailOverrides,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatbotConfigResponse {
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub activo: bool,
    pub connection_status: String,
    pub display_name: Option<String>,
    pub language: String,
    pub tone: Option<String>,
    pub greeting: Option<String>,
    pub system_prompt: Option<String>,
    pub faqs: Option<Vec<FaqEntry>>,
    pub policies: Option<String>,
    pub sender_policy: String,
    pub allowlist: Option<Vec<String>>,
    pub capabilities: Capabilities,
    pub handoff_keywords: Option<Vec<String>>,
    pub history_limit: i32,
    pub retention_days: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Partial update request. All fields optional.
///
/// Validation (enforced in service layer):
/// - `display_name`: 1–100 characters
/// - tone: 1–50 characters
/// - faqs: max 50 entries, question max 200 chars, answer max 1000 chars
/// - policies: max 5000 characters
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatbotConfigUpdateRequest {
    pub activo: Option<bool>,
    pub display_name: Option<String>,
    pub language: Option<String>,
    pub tone: Option<String>,
    pub greeting: Option<String>,
    pub system_prompt: Option<String>,
    pub faqs: Option<Vec<FaqEntry>>,
    pub policies: Option<String>,
    pub sender_policy: Option<String>,
    pub allowlist: Option<Vec<String>>,
    pub capabilities: Option<Capabilities>,
    pub handoff_keywords: Option<Vec<String>>,
    pub history_limit: Option<i32>,
    pub retention_days: Option<i32>,
}

// --- Webhook DTOs ---

/// Incoming message from Baileys sidecar via internal webhook.
/// Authenticated via X-Internal-Token header.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingWebhookPayload {
    pub realm_id: Uuid,
    pub sender_phone: String,
    pub message_type: String,
    pub content: String,
    pub caption: Option<String>,
    pub message_id: String,
    pub timestamp: i64,
    #[serde(default)]
    pub session_phone: Option<String>,
}

/// Request to send a message via Baileys service.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub recipient_phone: String,
    pub content: String,
    pub message_type: String,
}

// --- Conversation DTOs ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    pub id: Uuid,
    pub sender_phone: String,
    pub inquilino_id: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub message_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListResponse {
    pub sender_phone: String,
    pub inquilino_id: Option<Uuid>,
    pub last_message: String,
    pub last_message_at: DateTime<Utc>,
    pub message_count: i64,
}

// --- Receipt DTOs ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptExtractionResponse {
    pub id: Uuid,
    pub organizacion_id: Uuid,
    pub conversation_id: Uuid,
    pub inquilino_id: Option<Uuid>,
    pub contrato_id: Option<Uuid>,
    pub bank: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub date: Option<String>,
    pub reference: Option<String>,
    pub sender_name: Option<String>,
    pub recipient: Option<String>,
    pub confidence: String,
    pub status: String,
    pub confirmed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptConfirmRequest {
    pub contrato_id: Option<Uuid>,
    pub rejection_reason: Option<String>,
}

// --- Balance Query DTOs ---

/// Individual payment detail in a balance response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetail {
    pub amount: Decimal,
    pub currency: String,
    pub formatted_amount: String,
    pub due_date: chrono::NaiveDate,
    pub status: String,
}

/// Per-currency total in a balance response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyTotal {
    pub currency: String,
    pub total: Decimal,
    pub formatted_total: String,
}

/// Full balance response with individual payments and per-currency totals.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    pub payments: Vec<PaymentDetail>,
    pub totals: Vec<CurrencyTotal>,
}

/// Formats a monetary amount with the appropriate currency symbol and two decimal places.
///
/// - DOP → "RD$1,500.00"
/// - USD → "US$500.00"
pub fn format_currency(amount: Decimal, currency: &str) -> String {
    let symbol = match currency {
        "DOP" => "RD$",
        "USD" => "US$",
        _ => "",
    };

    // Format with two decimal places and thousands separators
    let abs_amount = if amount.is_sign_negative() {
        -amount
    } else {
        amount
    };

    // Split into integer and fractional parts
    let scaled = abs_amount.round_dp(2).to_string();
    let parts: Vec<&str> = scaled.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = if parts.len() > 1 {
        format!("{:0<2}", &parts[1][..parts[1].len().min(2)])
    } else {
        "00".to_string()
    };

    // Add thousands separators
    let integer_with_commas = add_thousands_separator(integer_part);

    if amount.is_sign_negative() {
        format!("-{symbol}{integer_with_commas}.{decimal_part}")
    } else {
        format!("{symbol}{integer_with_commas}.{decimal_part}")
    }
}

/// Adds comma thousands separators to an integer string.
fn add_thousands_separator(s: &str) -> String {
    let len = s.len();
    if len <= 3 {
        return s.to_string();
    }

    let mut result = String::with_capacity(len + len / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }

    result
}

// --- Test Chat DTOs ---

/// Request body for the test chat endpoint.
/// Includes the test message and optionally the current (possibly unsaved) config
/// so the AI pipeline uses the draft persona/capabilities/FAQs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestChatRequest {
    pub message: String,
    /// Optional config override — if provided, the test uses these settings
    /// instead of the persisted org config.
    pub config_override: Option<TestChatConfigOverride>,
    /// Conversation history from the test UI (role + content pairs).
    /// Allows multi-turn context in the test chat sandbox.
    #[serde(default)]
    pub history: Vec<TestChatHistoryEntry>,
}

/// A single message in the test chat conversation history.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestChatHistoryEntry {
    pub role: String,
    pub content: String,
}

/// Optional configuration override for test chat.
/// Mirrors the persona/capabilities/knowledge fields from the config wizard.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestChatConfigOverride {
    pub display_name: Option<String>,
    pub language: Option<String>,
    pub tone: Option<String>,
    pub greeting: Option<String>,
    pub system_prompt: Option<String>,
    pub faqs: Option<Vec<FaqEntry>>,
    pub policies: Option<String>,
    pub capabilities: Option<Capabilities>,
    pub handoff_keywords: Option<Vec<String>>,
    pub history_limit: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestChatResponse {
    pub reply: String,
    pub tools_invoked: Vec<String>,
}

// --- Connection Status ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusResponse {
    pub status: String,
    pub connected_phone: Option<String>,
    pub connected_at: Option<DateTime<Utc>>,
}

// --- Handoff DTOs ---

/// Request body for clearing a human handoff on a conversation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearHandoffRequest {
    pub sender_phone: String,
}

/// Response after clearing or setting handoff status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffStatusResponse {
    pub organizacion_id: Uuid,
    pub sender_phone: String,
    pub handoff_status: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn incoming_webhook_payload_deserializes_camel_case() {
        let json = serde_json::json!({
            "realmId": "550e8400-e29b-41d4-a716-446655440000",
            "senderPhone": "+18091234567",
            "messageType": "text",
            "content": "Hola, cuánto debo?",
            "messageId": "wamid.abc123",
            "timestamp": 1_715_200_000_i64
        });
        let payload: IncomingWebhookPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.sender_phone, "+18091234567");
        assert_eq!(payload.message_type, "text");
        assert_eq!(payload.timestamp, 1_715_200_000);
        assert!(payload.caption.is_none());
    }

    #[test]
    fn incoming_webhook_payload_with_image_and_caption() {
        let json = serde_json::json!({
            "realmId": "550e8400-e29b-41d4-a716-446655440000",
            "senderPhone": "+18091234567",
            "messageType": "image",
            "content": "base64encodeddata",
            "caption": "Mi recibo de pago",
            "messageId": "wamid.img456",
            "timestamp": 1_715_200_100_i64
        });
        let payload: IncomingWebhookPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.message_type, "image");
        assert_eq!(payload.caption.as_deref(), Some("Mi recibo de pago"));
    }

    #[test]
    fn chatbot_config_update_request_partial_fields() {
        let json = serde_json::json!({
            "displayName": "Mi Chatbot",
            "tone": "amigable"
        });
        let req: ChatbotConfigUpdateRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.display_name.as_deref(), Some("Mi Chatbot"));
        assert_eq!(req.tone.as_deref(), Some("amigable"));
        assert!(req.activo.is_none());
        assert!(req.faqs.is_none());
        assert!(req.policies.is_none());
    }

    #[test]
    fn chatbot_config_response_serializes_camel_case() {
        let now = Utc::now();
        let response = ChatbotConfigResponse {
            id: Uuid::new_v4(),
            organizacion_id: Uuid::new_v4(),
            activo: false,
            connection_status: "disconnected".to_string(),
            display_name: Some("Asistente".to_string()),
            language: "es-DO".to_string(),
            tone: Some("profesional".to_string()),
            greeting: None,
            system_prompt: None,
            faqs: None,
            policies: None,
            sender_policy: "tenants_only".to_string(),
            allowlist: None,
            capabilities: Capabilities {
                receipt_ocr: true,
                balance_queries: true,
                payment_reminders: false,
                maintenance_requests: true,
                human_handoff: false,
            },
            handoff_keywords: None,
            history_limit: 10,
            retention_days: 90,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("organizacionId").is_some());
        assert!(json.get("connectionStatus").is_some());
        assert!(json.get("displayName").is_some());
        assert!(json.get("senderPolicy").is_some());
        assert!(json.get("historyLimit").is_some());
        assert!(json.get("retentionDays").is_some());
        assert!(json.get("createdAt").is_some());
        assert_eq!(json["activo"], false);
        assert_eq!(json["language"], "es-DO");
    }

    #[test]
    fn test_chat_request_deserializes() {
        let json = serde_json::json!({
            "message": "Cuánto debo este mes?"
        });
        let req: TestChatRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.message, "Cuánto debo este mes?");
    }

    #[test]
    fn test_chat_response_serializes_camel_case() {
        let response = TestChatResponse {
            reply: "Su balance pendiente es RD$5,000.00".to_string(),
            tools_invoked: vec!["query_balance".to_string()],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("toolsInvoked").is_some());
        assert_eq!(json["reply"], "Su balance pendiente es RD$5,000.00");
    }

    #[test]
    fn connection_status_response_serializes_camel_case() {
        let response = ConnectionStatusResponse {
            status: "connected".to_string(),
            connected_phone: Some("+18091234567".to_string()),
            connected_at: Some(Utc::now()),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("connectedPhone").is_some());
        assert!(json.get("connectedAt").is_some());
        assert_eq!(json["status"], "connected");
    }

    #[test]
    fn send_message_request_serializes_camel_case() {
        let req = SendMessageRequest {
            recipient_phone: "+18091234567".to_string(),
            content: "Hola!".to_string(),
            message_type: "text".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("recipientPhone").is_some());
        assert!(json.get("messageType").is_some());
        assert_eq!(json["content"], "Hola!");
    }

    #[test]
    fn receipt_confirm_request_deserializes() {
        let json = serde_json::json!({
            "contratoId": "550e8400-e29b-41d4-a716-446655440000",
            "rejectionReason": "Monto incorrecto"
        });
        let req: ReceiptConfirmRequest = serde_json::from_value(json).unwrap();
        assert!(req.contrato_id.is_some());
        assert_eq!(req.rejection_reason.as_deref(), Some("Monto incorrecto"));
    }

    // --- AgentConfig tests ---

    #[test]
    fn agent_config_resolve_defaults() {
        let config = AgentConfig::default();
        let resolved = config.resolve();
        assert_eq!(resolved.max_turns, 5);
        assert_eq!(resolved.temperature, None);
        assert_eq!(resolved.max_tokens, None);
        assert_eq!(
            resolved.tool_registration,
            ToolRegistrationStrategy::Selective
        );
        assert_eq!(resolved.guardrails.output_safety_enabled, Some(true));
    }

    #[test]
    fn agent_config_resolve_clamps_max_turns() {
        let config = AgentConfig {
            max_turns: Some(0),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_turns, 1);

        let config = AgentConfig {
            max_turns: Some(20),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_turns, 15);

        let config = AgentConfig {
            max_turns: Some(7),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_turns, 7);
    }

    #[test]
    fn agent_config_resolve_filters_temperature() {
        let config = AgentConfig {
            temperature: Some(1.5),
            ..Default::default()
        };
        assert_eq!(config.resolve().temperature, Some(1.5));

        let config = AgentConfig {
            temperature: Some(-0.1),
            ..Default::default()
        };
        assert_eq!(config.resolve().temperature, None);

        let config = AgentConfig {
            temperature: Some(2.1),
            ..Default::default()
        };
        assert_eq!(config.resolve().temperature, None);

        // Boundary: exactly 0.0 and 2.0 are valid
        let config = AgentConfig {
            temperature: Some(0.0),
            ..Default::default()
        };
        assert_eq!(config.resolve().temperature, Some(0.0));

        let config = AgentConfig {
            temperature: Some(2.0),
            ..Default::default()
        };
        assert_eq!(config.resolve().temperature, Some(2.0));
    }

    #[test]
    fn agent_config_resolve_filters_max_tokens() {
        let config = AgentConfig {
            max_tokens: Some(1024),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_tokens, Some(1024));

        let config = AgentConfig {
            max_tokens: Some(0),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_tokens, None);

        let config = AgentConfig {
            max_tokens: Some(5000),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_tokens, None);

        // Boundary: exactly 1 and 4096 are valid
        let config = AgentConfig {
            max_tokens: Some(1),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_tokens, Some(1));

        let config = AgentConfig {
            max_tokens: Some(4096),
            ..Default::default()
        };
        assert_eq!(config.resolve().max_tokens, Some(4096));
    }

    #[test]
    fn agent_config_deserializes_camel_case() {
        let json = serde_json::json!({
            "maxTurns": 10,
            "temperature": 0.7,
            "maxTokens": 2048,
            "toolRegistration": "all_with_hook_gating",
            "guardrails": {
                "maxReceiptAmountDop": 50000.0,
                "outputSafetyEnabled": false
            }
        });
        let config: AgentConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.max_turns, Some(10));
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_tokens, Some(2048));
        assert_eq!(
            config.tool_registration,
            Some(ToolRegistrationStrategy::AllWithHookGating)
        );
        let guardrails = config.guardrails.unwrap();
        assert_eq!(guardrails.max_receipt_amount_dop, Some(50000.0));
        assert_eq!(guardrails.output_safety_enabled, Some(false));
    }

    #[test]
    fn agent_config_deserializes_empty_object() {
        let json = serde_json::json!({});
        let config: AgentConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.max_turns, None);
        assert_eq!(config.temperature, None);
        assert_eq!(config.max_tokens, None);
        assert_eq!(config.tool_registration, None);
        assert!(config.guardrails.is_none());
    }

    #[test]
    fn tool_registration_strategy_serializes_snake_case() {
        let selective = serde_json::to_value(ToolRegistrationStrategy::Selective).unwrap();
        assert_eq!(selective, "selective");

        let all = serde_json::to_value(ToolRegistrationStrategy::AllWithHookGating).unwrap();
        assert_eq!(all, "all_with_hook_gating");
    }

    #[test]
    fn guardrail_overrides_default() {
        let overrides = GuardrailOverrides::default();
        assert_eq!(overrides.max_receipt_amount_dop, None);
        assert_eq!(overrides.max_receipt_amount_usd, None);
        assert!(overrides.blocked_patterns.is_none());
        assert_eq!(overrides.output_safety_enabled, Some(true));
    }
}
