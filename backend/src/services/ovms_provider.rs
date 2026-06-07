#![allow(clippy::doc_markdown)]
//! Custom Rig `CompletionModel` implementation for OpenVINO Model Server (OVMS).
//!
//! OVMS exposes an OpenAI-compatible `/v3/chat/completions` endpoint but omits
//! the `id` field from responses (documented as unsupported). This module
//! provides a thin adapter that:
//!
//! 1. Converts Rig's `CompletionRequest` into the OVMS request format
//!    (including `tools`/`tool_choice` for function calling).
//! 2. Deserializes OVMS responses tolerating the missing `id` field.
//! 3. Returns Rig's `CompletionResponse` with `tool_calls` properly parsed,
//!    enabling the full agent loop (multi-turn tool calling, hooks, sub-agents).

use rig::completion::{self, CompletionError, CompletionRequest};
use rig::message::{self, AssistantContent, UserContent};
use rig::one_or_many::OneOrMany;
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse, StreamingResult};
use serde::{Deserialize, Serialize};

use eventsource_stream::Eventsource;
use futures_util::StreamExt;

// =============================================================================
// OVMS-specific request/response types
// =============================================================================

/// OpenAI-compatible chat message for OVMS requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum OvmsMessage {
    System {
        content: String,
    },
    User {
        content: serde_json::Value,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Vec::is_empty",
            deserialize_with = "deserialize_null_or_vec"
        )]
        tool_calls: Vec<OvmsToolCall>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

/// Tool call in OVMS response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvmsToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OvmsFunction,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvmsFunction {
    pub name: String,
    /// Arguments as a JSON string (OVMS returns stringified JSON).
    #[serde(deserialize_with = "deserialize_arguments")]
    pub arguments: serde_json::Value,
}

/// Tool definition sent to OVMS.
#[derive(Debug, Clone, Serialize)]
pub struct OvmsToolDef {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OvmsFunctionDef,
}

/// Function definition within a tool.
#[derive(Debug, Clone, Serialize)]
pub struct OvmsFunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// The full request body sent to OVMS.
#[derive(Debug, Serialize)]
pub struct OvmsRequest {
    pub model: String,
    pub messages: Vec<OvmsMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<OvmsToolDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    pub stream: bool,
}

/// OVMS chat completion response (tolerates missing `id` field).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvmsCompletionResponse {
    /// OVMS omits this field — we default to empty string.
    #[serde(default)]
    pub id: String,
    pub choices: Vec<OvmsChoice>,
    pub created: u64,
    pub model: String,
    pub object: String,
    #[serde(default)]
    pub usage: Option<OvmsUsage>,
}

/// A single choice in the OVMS response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvmsChoice {
    pub index: usize,
    pub message: OvmsResponseMessage,
    pub finish_reason: Option<String>,
}

/// The assistant message in a response choice.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvmsResponseMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_or_vec")]
    pub tool_calls: Vec<OvmsToolCall>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvmsUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

// =============================================================================
// Custom CompletionModel implementation
// =============================================================================

/// A Rig-compatible completion model that talks to OVMS.
#[derive(Clone)]
pub struct OvmsCompletionModel {
    pub(crate) client: reqwest::Client,
    pub(crate) model_name: String,
    pub(crate) endpoint: String,
    pub(crate) api_key: Option<String>,
}

impl OvmsCompletionModel {
    /// Creates a new OVMS completion model.
    ///
    /// `endpoint` should be the base URL including the version path,
    /// e.g. `http://ovms:8000/v3`. The `/chat/completions` suffix is appended
    /// automatically.
    pub fn new(
        model_name: impl Into<String>,
        endpoint: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            model_name: model_name.into(),
            endpoint: endpoint.into(),
            api_key,
        }
    }
}

impl completion::CompletionModel for OvmsCompletionModel {
    type Response = OvmsCompletionResponse;
    type StreamingResponse = OvmsCompletionResponse;
    type Client = ();

    fn make(_client: &Self::Client, model: impl Into<String>) -> Self {
        // Fallback constructor — prefer OvmsCompletionModel::new() with explicit endpoint.
        Self {
            client: reqwest::Client::new(),
            model_name: model.into(),
            endpoint: String::new(),
            api_key: None,
        }
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<Self::Response>, CompletionError> {
        // Convert Rig CompletionRequest → OVMS request format
        let ovms_request = build_ovms_request(&self.model_name, &request);

        let url = format!("{}/chat/completions", self.endpoint);

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(key) = &self.api_key {
            request_builder = request_builder.header("Authorization", format!("Bearer {key}"));
        }

        let response = request_builder
            .json(&ovms_request)
            .send()
            .await
            .map_err(|e| CompletionError::ProviderError(format!("Error conectando a OVMS: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CompletionError::ProviderError(format!(
                "OVMS respondió con estado {status}: {body}"
            )));
        }

        let ovms_response: OvmsCompletionResponse = response.json().await.map_err(|e| {
            CompletionError::ProviderError(format!("Error parseando respuesta OVMS: {e}"))
        })?;

        // Convert OVMS response → Rig CompletionResponse
        ovms_response_to_rig(ovms_response)
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let mut ovms_request = build_ovms_request(&self.model_name, &request);
        ovms_request.stream = true;

        let url = format!("{}/chat/completions", self.endpoint);

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(key) = &self.api_key {
            request_builder = request_builder.header("Authorization", format!("Bearer {key}"));
        }

        let response = request_builder
            .json(&ovms_request)
            .send()
            .await
            .map_err(|e| {
                CompletionError::ProviderError(format!("Error conectando a OVMS stream: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CompletionError::ProviderError(format!(
                "OVMS stream respondió con estado {status}: {body}"
            )));
        }

        let event_stream = response.bytes_stream().eventsource();

        let stream = event_stream.filter_map(|event_result| async move {
            match event_result {
                Ok(event) => {
                    let data = event.data.trim().to_string();
                    if data == "[DONE]" {
                        return None;
                    }
                    Some(parse_sse_chunk(&data))
                }
                Err(e) => Some(Err(CompletionError::ProviderError(format!(
                    "Error en SSE stream: {e}"
                )))),
            }
        });

        let pinned: StreamingResult<OvmsCompletionResponse> = Box::pin(stream);
        Ok(StreamingCompletionResponse::stream(pinned))
    }
}

impl completion::GetTokenUsage for OvmsCompletionResponse {
    fn token_usage(&self) -> Option<completion::Usage> {
        self.usage.as_ref().map(|u| completion::Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            cached_input_tokens: 0,
            cache_creation_input_tokens: 0,
            reasoning_tokens: 0,
        })
    }
}

// =============================================================================
// Conversion: Rig CompletionRequest → OVMS Request
// =============================================================================

/// Builds an OVMS-compatible request from Rig's `CompletionRequest`.
fn build_ovms_request(model_name: &str, request: &CompletionRequest) -> OvmsRequest {
    let mut messages = Vec::new();

    // System prompt (preamble)
    if let Some(preamble) = &request.preamble {
        messages.push(OvmsMessage::System {
            content: preamble.clone(),
        });
    }

    // Chat history + prompt (all in chat_history, last message is the prompt)
    for msg in request.chat_history.iter() {
        messages.push(rig_message_to_ovms(msg));
    }

    // Convert tool definitions
    let tools: Vec<OvmsToolDef> = request
        .tools
        .iter()
        .map(|td| OvmsToolDef {
            tool_type: "function".to_string(),
            function: OvmsFunctionDef {
                name: td.name.clone(),
                description: td.description.clone(),
                parameters: td.parameters.clone(),
            },
        })
        .collect();

    // Convert tool_choice
    let tool_choice = request.tool_choice.as_ref().map(|tc| match tc {
        message::ToolChoice::None => serde_json::json!("none"),
        message::ToolChoice::Auto => serde_json::json!("auto"),
        message::ToolChoice::Required => serde_json::json!("required"),
        message::ToolChoice::Specific { function_names } => function_names.first().map_or_else(
            || serde_json::json!("auto"),
            |name| {
                serde_json::json!({
                    "type": "function",
                    "function": { "name": name }
                })
            },
        ),
    });

    OvmsRequest {
        model: model_name.to_string(),
        messages,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        tools,
        tool_choice,
        stream: false,
    }
}

/// Converts a Rig `Message` to an OVMS message.
fn rig_message_to_ovms(msg: &message::Message) -> OvmsMessage {
    match msg {
        message::Message::System { content } => OvmsMessage::System {
            content: content.clone(),
        },
        message::Message::User { content } => {
            let content_value = user_content_to_json(content);
            OvmsMessage::User {
                content: content_value,
            }
        }
        message::Message::Assistant { content, .. } => {
            let mut text_parts = Vec::new();
            let mut tool_calls = Vec::new();

            for item in content.iter() {
                match item {
                    AssistantContent::Text(text) => {
                        text_parts.push(text.text.clone());
                    }
                    AssistantContent::ToolCall(tc) => {
                        tool_calls.push(OvmsToolCall {
                            id: tc.id.clone(),
                            call_type: "function".to_string(),
                            function: OvmsFunction {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        });
                    }
                    _ => {}
                }
            }

            let content_str = if text_parts.is_empty() {
                None
            } else {
                Some(text_parts.join("\n"))
            };

            OvmsMessage::Assistant {
                content: content_str,
                tool_calls,
            }
        }
    }
}

/// Converts user content to a JSON value suitable for OVMS.
fn user_content_to_json(content: &OneOrMany<UserContent>) -> serde_json::Value {
    // Check if it's simple text-only content
    let items: Vec<&UserContent> = content.iter().collect();

    if items.len() == 1 {
        if let UserContent::Text(text) = items[0] {
            return serde_json::Value::String(text.text.clone());
        }
    }

    // For tool results, extract the text content
    let mut parts = Vec::new();
    for item in items {
        match item {
            UserContent::Text(text) => {
                parts.push(serde_json::json!({
                    "type": "text",
                    "text": text.text
                }));
            }
            UserContent::ToolResult(tr) => {
                // Tool results are sent as a separate "tool" role message in
                // OpenAI format, but Rig wraps them as User content. We convert
                // them to text for OVMS compatibility.
                let result_text = tr
                    .content
                    .iter()
                    .map(|c| match c {
                        message::ToolResultContent::Text(t) => t.text.clone(),
                        message::ToolResultContent::Image(_) => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                parts.push(serde_json::json!({
                    "type": "text",
                    "text": result_text
                }));
            }
            _ => {}
        }
    }

    if parts.len() == 1 {
        if let Some(text) = parts[0].get("text").and_then(|v| v.as_str()) {
            return serde_json::Value::String(text.to_string());
        }
    }

    serde_json::Value::Array(parts)
}

// =============================================================================
// SSE streaming chunk parsing
// =============================================================================

#[derive(Debug, Deserialize)]
struct OvmsStreamChunk {
    choices: Vec<OvmsStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OvmsStreamChoice {
    delta: OvmsStreamDelta,
}

#[derive(Debug, Deserialize)]
struct OvmsStreamDelta {
    #[serde(default)]
    content: Option<String>,
}

fn parse_sse_chunk(
    data: &str,
) -> Result<RawStreamingChoice<OvmsCompletionResponse>, CompletionError> {
    let chunk: OvmsStreamChunk = serde_json::from_str(data)
        .map_err(|e| CompletionError::ProviderError(format!("Error parseando chunk SSE: {e}")))?;

    let text = chunk
        .choices
        .first()
        .and_then(|c| c.delta.content.clone())
        .unwrap_or_default();

    Ok(RawStreamingChoice::Message(text))
}

// =============================================================================
// Conversion: OVMS Response → Rig CompletionResponse
// =============================================================================

/// Converts an OVMS response into Rig's `CompletionResponse`.
fn ovms_response_to_rig(
    response: OvmsCompletionResponse,
) -> Result<completion::CompletionResponse<OvmsCompletionResponse>, CompletionError> {
    let choice = response.choices.first().ok_or_else(|| {
        CompletionError::ResponseError("OVMS response contained no choices".to_string())
    })?;

    let mut content = Vec::new();

    // Extract text content
    if let Some(text) = &choice.message.content {
        if !text.is_empty() {
            content.push(AssistantContent::text(text));
        }
    }

    // Extract tool calls
    for tc in &choice.message.tool_calls {
        content.push(AssistantContent::tool_call(
            &tc.id,
            &tc.function.name,
            tc.function.arguments.clone(),
        ));
    }

    let choice = OneOrMany::many(content).map_err(|_| {
        CompletionError::ResponseError(
            "OVMS response contained no message or tool call (empty)".to_string(),
        )
    })?;

    let usage = response.usage.as_ref().map_or(
        completion::Usage {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cached_input_tokens: 0,
            cache_creation_input_tokens: 0,
            reasoning_tokens: 0,
        },
        |u| completion::Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            cached_input_tokens: 0,
            cache_creation_input_tokens: 0,
            reasoning_tokens: 0,
        },
    );

    Ok(completion::CompletionResponse {
        choice,
        usage,
        raw_response: response,
        message_id: None,
    })
}

// =============================================================================
// Serde helpers
// =============================================================================

/// Deserializes `null` or missing arrays as empty `Vec`.
fn deserialize_null_or_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt: Option<Vec<T>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Deserializes tool call arguments that may be a JSON string or a JSON object.
fn deserialize_arguments<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => serde_json::from_str(&s).map_err(|e| {
            serde::de::Error::custom(format!("Failed to parse tool arguments JSON string: {e}"))
        }),
        other => Ok(other),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unreadable_literal, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_ovms_response_without_id() {
        let json = r#"{
            "choices": [{
                "finish_reason": "stop",
                "index": 0,
                "message": {
                    "content": "Hello! How can I help you?",
                    "role": "assistant"
                }
            }],
            "created": 1716825108,
            "model": "qwen3.6",
            "object": "chat.completion",
            "usage": {
                "completion_tokens": 10,
                "prompt_tokens": 22,
                "total_tokens": 32
            }
        }"#;

        let response: OvmsCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, ""); // defaults to empty
        assert_eq!(response.model, "qwen3.6");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("Hello! How can I help you?")
        );
        assert!(response.choices[0].message.tool_calls.is_empty());
    }

    #[test]
    fn deserialize_ovms_response_with_tool_calls() {
        let json = r#"{
            "choices": [{
                "finish_reason": "tool_calls",
                "index": 0,
                "message": {
                    "content": null,
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "query_balance",
                            "arguments": "{\"inquilino_id\":\"uuid-1\",\"organizacion_id\":\"uuid-2\"}"
                        }
                    }]
                }
            }],
            "created": 1716825108,
            "model": "qwen3.6",
            "object": "chat.completion",
            "usage": {
                "completion_tokens": 25,
                "prompt_tokens": 100,
                "total_tokens": 125
            }
        }"#;

        let response: OvmsCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.choices[0].finish_reason.as_deref(),
            Some("tool_calls")
        );
        assert_eq!(response.choices[0].message.tool_calls.len(), 1);

        let tc = &response.choices[0].message.tool_calls[0];
        assert_eq!(tc.id, "call_abc123");
        assert_eq!(tc.function.name, "query_balance");
        assert_eq!(
            tc.function.arguments,
            serde_json::json!({"inquilino_id": "uuid-1", "organizacion_id": "uuid-2"})
        );
    }

    #[test]
    fn deserialize_ovms_response_with_null_tool_calls() {
        let json = r#"{
            "choices": [{
                "finish_reason": "stop",
                "index": 0,
                "message": {
                    "content": "Sure, let me check.",
                    "role": "assistant",
                    "tool_calls": null
                }
            }],
            "created": 1716825108,
            "model": "qwen3.6",
            "object": "chat.completion",
            "usage": {
                "completion_tokens": 5,
                "prompt_tokens": 10,
                "total_tokens": 15
            }
        }"#;

        let response: OvmsCompletionResponse = serde_json::from_str(json).unwrap();
        assert!(response.choices[0].message.tool_calls.is_empty());
    }

    #[test]
    fn ovms_response_converts_to_rig_completion() {
        let response = OvmsCompletionResponse {
            id: String::new(),
            choices: vec![OvmsChoice {
                index: 0,
                message: OvmsResponseMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: vec![],
                },
                finish_reason: Some("stop".to_string()),
            }],
            created: 1716825108,
            model: "qwen3.6".to_string(),
            object: "chat.completion".to_string(),
            usage: Some(OvmsUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let rig_response = ovms_response_to_rig(response).unwrap();
        assert_eq!(rig_response.usage.input_tokens, 10);
        assert_eq!(rig_response.usage.output_tokens, 5);
    }

    #[test]
    fn ovms_response_with_tool_calls_converts_to_rig() {
        let response = OvmsCompletionResponse {
            id: String::new(),
            choices: vec![OvmsChoice {
                index: 0,
                message: OvmsResponseMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: vec![OvmsToolCall {
                        id: "call_1".to_string(),
                        call_type: "function".to_string(),
                        function: OvmsFunction {
                            name: "query_balance".to_string(),
                            arguments: serde_json::json!({"inquilino_id": "abc"}),
                        },
                    }],
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            created: 1716825108,
            model: "qwen3.6".to_string(),
            object: "chat.completion".to_string(),
            usage: None,
        };

        let rig_response = ovms_response_to_rig(response).unwrap();
        // Should contain a tool call
        let first = rig_response.choice.first();
        match first {
            AssistantContent::ToolCall(tc) => {
                assert_eq!(tc.function.name, "query_balance");
            }
            _ => panic!("Expected tool call content"),
        }
    }
}
