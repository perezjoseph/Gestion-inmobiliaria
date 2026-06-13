# Streaming and Completions — Detailed Reference

## Completion Traits Hierarchy

| Trait | Level | Returns | Use case |
|-------|-------|---------|----------|
| `Prompt` | High | `String` | Simple one-shot, no history |
| `Chat` | High | `String` | Conversation with message history |
| `TypedPrompt` | High | `T: Deserialize + JsonSchema` | Structured output directly |
| `Completion` | Low | `CompletionRequestBuilder` | Full control over request params |
| `StreamingPrompt` | High | `Stream<StreamedAssistantContent>` | Incremental token delivery |
| `StreamingChat` | High | `Stream<StreamedAssistantContent>` | Streaming with history |
| `StreamingCompletion` | Low | `StreamingCompletionResponse` | Full streaming control |

## Response Types

```rust
pub struct CompletionResponse<T> {
    pub choice: OneOrMany<AssistantContent>,
    pub raw_response: T,
}

pub enum AssistantContent {
    Text(Text),           // Plain text response
    ToolCall(ToolCall),   // Model wants to call a tool
    Reasoning(Reasoning), // Chain-of-thought (models that support it)
}

pub struct ToolCall {
    pub id: String,
    pub function: ToolFunction,
}

pub struct ToolFunction {
    pub name: String,
    pub arguments: serde_json::Value,
}
```

## Message Types

```rust
pub enum Message {
    User { content: OneOrMany<UserContent> },
    Assistant { content: OneOrMany<AssistantContent> },
}

pub enum UserContent {
    Text(Text),
    ToolResult(ToolResult),
    Image(Image),
    Audio(Audio),
    Document(Document),
    Video(Video),
}
```

Construct messages:
```rust
Message::user("Hello")
Message::assistant("Hi there")
```

## Token Usage

```rust
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}
```

Implement `GetTokenUsage` on your provider's raw response type to expose metrics.

## Low-Level Completion Request

```rust
let response = model
    .completion_request("Complex query")
    .preamble("Expert system instructions")
    .temperature(0.8)
    .max_tokens(2000)
    .documents(context_docs)
    .tools(available_tools)
    .send()
    .await?;

// Access the response
match &response.choice {
    OneOrMany::One(content) => { /* single response */ }
    OneOrMany::Many(contents) => { /* multiple content items */ }
}
```

## Streaming

### Basic streaming

```rust
use rig::streaming::StreamingPrompt;
use futures::StreamExt;

let mut stream = agent.stream_prompt("Tell me a story").await?;

while let Some(chunk) = stream.next().await {
    match chunk? {
        StreamedAssistantContent::Text(text) => print!("{text}"),
        StreamedAssistantContent::ToolCallDelta(delta) => {
            // Buffer these until complete, then execute
            match delta {
                ToolCallDeltaContent::Name(name) => { /* tool name */ }
                ToolCallDeltaContent::Arguments(args) => { /* partial JSON args */ }
            }
        }
        StreamedAssistantContent::FinalUsage(usage) => {
            // Token counts for the entire completion (sent at end)
        }
    }
}
```

### Streaming with chat history

```rust
use rig::streaming::StreamingChat;

let mut stream = agent.stream_chat("Continue the story", chat_history).await?;
```

### Multi-turn streaming

When agents have tools, multi-turn streaming produces events for the full loop:

```rust
pub enum MultiTurnStreamItem {
    UserContent(StreamedUserContent),      // e.g., tool results sent back
    AssistantContent(StreamedAssistantContent), // text, tool calls, usage
}
```

### Convenience: print to stdout

```rust
use rig::streaming::stream_to_stdout;

let stream = agent.stream_prompt("Hello!").await?;
stream_to_stdout(stream).await?;
// Prints text chunks, ignores tool call deltas
```

### PauseControl

For interactive UIs where the user can pause/resume streaming:

```rust
use rig::streaming::PauseControl;

let pause = PauseControl::new();
let pause_clone = pause.clone();

// In another task:
pause_clone.pause();
// ... later:
pause_clone.resume();
```

## Error Types

```rust
pub enum CompletionError {
    HttpError(reqwest::Error),
    JsonError(serde_json::Error),
    RequestError(Box<dyn Error>),
    ResponseError(String),
    ProviderError(String),
}

pub enum PromptError {
    CompletionError(CompletionError),
    MaxDepthError, // multi-turn exceeded max turns
    PromptCancelled, // hook cancelled via CancelSignal
}

pub enum StructuredOutputError {
    CompletionError(CompletionError),
    JsonError(serde_json::Error),
}
```

## Best Practices

- **Error handling per chunk**: Each stream chunk can fail independently. Don't assume the whole stream succeeds or fails atomically.
- **Buffer tool call deltas**: Wait for the complete tool call before executing. Name comes first, then argument chunks.
- **Backpressure**: Use `PauseControl` or standard stream mechanisms when the consumer can't keep up.
- **FinalUsage**: Provides token counts for the entire completion, not per-chunk. Only available at stream end.
- **Reuse model instances**: `completion_model()` returns a lightweight handle. Create once, use many times.
