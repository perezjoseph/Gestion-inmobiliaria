---
inclusion: fileMatch
fileMatchPattern: ["**/ai_module.rs", "**/ovms_provider.rs"]
---

# Rig Framework Rules

Project-specific types and patterns for our OVMS-backed Rig integration. Rig's API changes between versions and has strict trait bounds (`GetTokenUsage`, `Clone + Unpin + Send + Sync + Serialize + DeserializeOwned`).

## Architecture

All LLM calls go through `OvmsCompletionModel` (implements `rig::completion::CompletionModel`). No direct HTTP calls to OVMS outside `ovms_provider.rs`. The OVMS endpoint is `/v3/chat/completions` and omits the `id` field in responses (by design, documented as unsupported).

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
4. OVMS response has no `id` field — always use `#[serde(default)]`
