use crate::services::api::{BASE_URL, api_delete, api_get, api_post, api_put};
use crate::types::chatbot::{
    BatchUpdateRequest, ChatbotConfigResponse, ChatbotConfigUpdateRequest,
    ConnectionStatusResponse, ConversationListItem, CreateGuidanceRuleRequest, GuidanceRule,
    ReceiptConfirmRequest, ReceiptExtractionResponse, ReceiptRejectRequest, TestChatRequest,
    TestChatResponse, UpdateGuidanceRuleRequest,
};

const CHATBOT_PATH: &str = "/chatbot";
const GUIDANCE_RULES_PATH: &str = "/chatbot/guidance-rules";

pub fn test_chat_stream_url() -> String {
    format!("{BASE_URL}{CHATBOT_PATH}/test/stream")
}

#[allow(clippy::future_not_send)]
pub async fn get_config() -> Result<ChatbotConfigResponse, String> {
    api_get(&format!("{CHATBOT_PATH}/config")).await
}

#[allow(clippy::future_not_send)]
pub async fn update_config(
    request: &ChatbotConfigUpdateRequest,
) -> Result<ChatbotConfigResponse, String> {
    api_put(&format!("{CHATBOT_PATH}/config"), request).await
}

#[allow(clippy::future_not_send)]
pub async fn connect() -> Result<ConnectionStatusResponse, String> {
    api_post(&format!("{CHATBOT_PATH}/connect"), &()).await
}

#[allow(clippy::future_not_send)]
pub async fn disconnect() -> Result<ConnectionStatusResponse, String> {
    api_post(&format!("{CHATBOT_PATH}/disconnect"), &()).await
}

#[allow(clippy::future_not_send)]
pub async fn status() -> Result<ConnectionStatusResponse, String> {
    api_get(&format!("{CHATBOT_PATH}/status")).await
}

#[allow(clippy::future_not_send)]
pub async fn test_chat(request: &TestChatRequest) -> Result<TestChatResponse, String> {
    api_post(&format!("{CHATBOT_PATH}/test"), request).await
}

#[allow(clippy::future_not_send)]
pub async fn list_conversations() -> Result<Vec<ConversationListItem>, String> {
    api_get(&format!("{CHATBOT_PATH}/conversations")).await
}

#[allow(clippy::future_not_send)]
pub async fn pending_receipts() -> Result<Vec<ReceiptExtractionResponse>, String> {
    api_get(&format!("{CHATBOT_PATH}/receipts/pending")).await
}

#[allow(clippy::future_not_send)]
pub async fn confirm_receipt(
    id: &str,
    request: &ReceiptConfirmRequest,
) -> Result<ReceiptExtractionResponse, String> {
    api_post(&format!("{CHATBOT_PATH}/receipts/{id}/confirm"), request).await
}

#[allow(clippy::future_not_send)]
pub async fn reject_receipt(
    id: &str,
    request: &ReceiptRejectRequest,
) -> Result<ReceiptExtractionResponse, String> {
    api_post(&format!("{CHATBOT_PATH}/receipts/{id}/reject"), request).await
}

#[allow(clippy::future_not_send)]
pub async fn create_guidance_rule(
    request: &CreateGuidanceRuleRequest,
) -> Result<GuidanceRule, String> {
    api_post(GUIDANCE_RULES_PATH, request).await
}

#[allow(clippy::future_not_send)]
pub async fn update_guidance_rule(
    id: &str,
    request: &UpdateGuidanceRuleRequest,
) -> Result<GuidanceRule, String> {
    api_put(&format!("{GUIDANCE_RULES_PATH}/{id}"), request).await
}

#[allow(clippy::future_not_send)]
pub async fn delete_guidance_rule(id: &str) -> Result<(), String> {
    api_delete(&format!("{GUIDANCE_RULES_PATH}/{id}")).await
}

#[allow(clippy::future_not_send)]
pub async fn batch_update_guidance_rules(
    request: &BatchUpdateRequest,
) -> Result<Vec<GuidanceRule>, String> {
    api_put(&format!("{GUIDANCE_RULES_PATH}/batch"), request).await
}
