//! `RentalGuardrailHook` — implements Rig's `PromptHook` trait for argument
//! validation, side-effect capture, and output safety filtering.
//!
//! This module provides the initial skeleton. Actual logic for `on_tool_call`,
//! `on_tool_result`, and `on_completion_response` will be added in later tasks.

use std::sync::{Arc, Mutex};

use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use uuid::Uuid;

use super::GuardrailConfig;
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
        _response: &rig::completion::CompletionResponse<M::Response>,
    ) -> HookAction {
        // Output safety filtering — implemented in task 6.8
        HookAction::Continue
    }

    async fn on_tool_call(
        &self,
        _tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
    ) -> ToolCallHookAction {
        // Argument validation — implemented in task 6.2
        ToolCallHookAction::Continue
    }

    async fn on_tool_result(
        &self,
        _tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        _result: &str,
    ) -> HookAction {
        // Side-effect capture — implemented in task 6.5
        HookAction::Continue
    }
}
