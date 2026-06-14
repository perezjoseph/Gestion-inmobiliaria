---
inclusion: fileMatch
fileMatchPattern: ["**/ai_module.rs", "**/ovms_provider.rs"]
---

# Rig Framework Rules

Project-specific types and patterns for our LLM Scaler-backed Rig integration. Rig's API changes between versions and has strict trait bounds (`GetTokenUsage`, `Clone + Unpin + Send + Sync + Serialize + DeserializeOwned`).

## Architecture

All LLM calls go through `OvmsCompletionModel` (implements `rig::completion::CompletionModel`). No direct HTTP calls to LLM Scaler outside `ovms_provider.rs`. The inference endpoint is `/v1/chat/completions` (OpenAI-compatible API served by [intel/llm-scaler](https://github.com/intel/llm-scaler) vLLM on Intel Arc Pro GPUs). The K8s service is `vllm-inference.realestate.svc.cluster.local:8000`. Container image: `intel/llm-scaler-vllm`. `#[serde(default)]` on response `id` field kept for safety.

## Key types

- `OvmsCompletionModel` — custom provider in `services/ovms_provider.rs`
- `AiModule` — high-level wrapper in `services/ai_module.rs` that composes system prompts and calls the model
- `CompletionRequest` — Rig's request struct (preamble, chat_history, tools, tool_choice)
- `rig::message::ToolChoice` — not in `rig::completion`, imported from `rig::message`
- `StreamingCompletionResponse::stream(pinned)` — constructor for streaming responses
- `RawStreamingChoice::Message(text)` — SSE text chunk type

## Checklist before changes

1. Verify trait bounds match rig-core 0.37 (check `CompletionModel`, `GetTokenUsage`)
2. Use `rig::message::ToolChoice` not `rig::completion::ToolChoice`
3. Streaming requires `eventsource-stream` for SSE parsing
4. Keep `#[serde(default)]` on response `id` field for backward compatibility
5. LLM Scaler uses standard vLLM OpenAI-compatible API — no custom request/response fields needed
