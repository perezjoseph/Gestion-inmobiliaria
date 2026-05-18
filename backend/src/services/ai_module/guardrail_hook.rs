//! `RentalGuardrailHook` — implements Rig's `PromptHook` trait for argument
//! validation, side-effect capture, and output safety filtering.
//!
//! This module provides the initial skeleton. Actual logic for `on_tool_call`,
//! `on_tool_result`, and `on_completion_response` will be added in later tasks.

use std::sync::{Arc, Mutex};

use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use uuid::Uuid;

use super::{CreateMaintenanceRequestInput, GuardrailConfig};
use crate::services::ai_module::tools::PaymentReceipt;

/// Guardrail hook that intercepts the Rig agent loop at four points:
/// - `on_completion_call`: before sending to LLM (no-op)
/// - `on_completion_response`: output safety filtering (later task 6.8)
/// - `on_tool_call`: argument validation (later task 6.2)
/// - `on_tool_result`: side-effect capture (later task 6.5)
#[derive(Clone)]
pub struct RentalGuardrailHook {
    /// Captured receipt from `extract_receipt` tool results.
    pub captured_receipt: Arc<Mutex<Option<PaymentReceipt>>>,
    /// Names of tools invoked during the agent loop, in order.
    pub tools_invoked: Arc<Mutex<Vec<String>>>,
    /// Organization ID for logging and context.
    pub organizacion_id: Uuid,
    /// Validation limits and blocked output patterns.
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
        // Skip if no blocked patterns configured
        if self.guardrail_config.blocked_output_patterns.is_empty() {
            return HookAction::Continue;
        }

        // Extract text content from response (read-only)
        let text = response
            .choice
            .iter()
            .filter_map(|c| match c {
                rig::message::AssistantContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Check against blocked patterns
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
                // Try to deserialize args to check description length
                if let Ok(parsed) = serde_json::from_str::<CreateMaintenanceRequestInput>(args) {
                    let desc_len = parsed.description.len(); // UTF-8 byte length
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
                // If deserialization fails, let Rig handle the error
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
        // Record tool invocation
        self.tools_invoked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(tool_name.to_string());

        // Capture receipt extraction side-effect
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
