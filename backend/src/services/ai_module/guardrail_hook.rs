use std::sync::{Arc, Mutex};

use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use uuid::Uuid;

use super::{CreateMaintenanceRequestInput, GuardrailConfig};
use crate::services::ai_module::tools::PaymentReceipt;

#[derive(Clone)]
pub struct RentalGuardrailHook {
    pub captured_receipt: Arc<Mutex<Option<PaymentReceipt>>>,
    pub tools_invoked: Arc<Mutex<Vec<String>>>,
    pub organizacion_id: Uuid,
    pub guardrail_config: GuardrailConfig,
}

impl<M> PromptHook<M> for RentalGuardrailHook
where
    M: CompletionModel,
{
    async fn on_completion_call(
        &self,
        _prompt: &rig::message::Message,
        _history: &[rig::message::Message],
    ) -> HookAction {
        HookAction::Continue
    }

    async fn on_completion_response(
        &self,
        _prompt: &rig::message::Message,
        response: &rig::completion::CompletionResponse<M::Response>,
    ) -> HookAction {
        if self.guardrail_config.blocked_output_patterns.is_empty() {
            return HookAction::Continue;
        }

        let text = response
            .choice
            .iter()
            .filter_map(|c| match c {
                rig::message::AssistantContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        for pattern in &self.guardrail_config.blocked_output_patterns {
            if pattern.is_match(&text) {
                tracing::warn!(
                    organizacion_id = %self.organizacion_id,
                    pattern = %pattern,
                    "Output safety check triggered"
                );
                return HookAction::Terminate {
                    reason: "Response blocked by safety filter".into(),
                };
            }
        }

        HookAction::Continue
    }

    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> ToolCallHookAction {
        match tool_name {
            "create_maintenance_request" => {
                if let Ok(parsed) = serde_json::from_str::<CreateMaintenanceRequestInput>(args) {
                    let desc_len = parsed.description.len();
                    if desc_len < 2 {
                        return ToolCallHookAction::Skip {
                            reason: "Description too short (min 2 chars)".into(),
                        };
                    }
                    if desc_len > self.guardrail_config.max_description_length {
                        return ToolCallHookAction::Skip {
                            reason: format!(
                                "Description too long (max {} chars)",
                                self.guardrail_config.max_description_length
                            ),
                        };
                    }
                }
                ToolCallHookAction::Continue
            }
            _ => ToolCallHookAction::Continue,
        }
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        result: &str,
    ) -> HookAction {
        self.tools_invoked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(tool_name.to_string());

        if tool_name == "extract_receipt" {
            if let Ok(receipt) = serde_json::from_str::<PaymentReceipt>(result) {
                *self
                    .captured_receipt
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(receipt);
            }
        }

        HookAction::Continue
    }
}
