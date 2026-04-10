use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

// --- Configuration DTOs ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FaqEntry {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub receipt_ocr: bool,
    pub balance_queries: bool,
    pub payment_reminders: bool,
    pub maintenance_requests: bool,
    pub human_handoff: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatbotConfigResponse {
    pub id: String,
    pub organizacion_id: String,
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatbotConfigUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faqs: Option<Vec<FaqEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowlist: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<i32>,
}

// --- Connection Status ---

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusResponse {
    pub status: String,
    pub qr_code: Option<String>,
    pub connected_phone: Option<String>,
    pub connected_at: Option<String>,
}

// --- Test Chat DTOs ---

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestChatConfigOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faqs: Option<Vec<FaqEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestChatRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_override: Option<TestChatConfigOverride>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestChatResponse {
    pub reply: String,
    pub tools_invoked: Vec<String>,
}

// --- Conversation DTOs ---

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListItem {
    pub sender_phone: String,
    pub inquilino_id: Option<String>,
    pub last_message: String,
    pub last_message_at: String,
    pub message_count: i64,
}

// --- Receipt DTOs ---

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptExtractionResponse {
    pub id: String,
    pub organizacion_id: String,
    pub conversation_id: String,
    pub inquilino_id: Option<String>,
    pub contrato_id: Option<String>,
    pub bank: Option<String>,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub amount: f64,
    pub currency: String,
    pub date: Option<String>,
    pub reference: Option<String>,
    pub sender_name: Option<String>,
    pub recipient: Option<String>,
    pub confidence: String,
    pub status: String,
    pub confirmed_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptConfirmRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contrato_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptRejectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}
