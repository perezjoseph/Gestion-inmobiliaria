# Tools and Agents — Detailed Reference

## Agent Builder Full API

```rust
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{Prompt, Chat};

let openai = openai::Client::from_env();

let agent = openai
    .agent("gpt-4o")
    .preamble("System prompt here")           // system message
    .temperature(0.7)                          // sampling temperature
    .max_tokens(1024)                          // max response tokens
    .context("Static context always included") // static document
    .tool(MyTool)                              // static tool
    .dynamic_context(3, vector_index)          // RAG: retrieve top-N docs
    .dynamic_tools(2, tool_index, toolset)     // RAG: retrieve top-N tools
    .additional_params(serde_json::json!({}))  // provider-specific params
    .tool_server(handle.clone())               // shared tool server
    .build();

// One-shot
let response = agent.prompt("Hello!").await?;

// Chat with history
let response = agent.chat("Continue", vec![
    Message::user("Previous message"),
    Message::assistant("Previous response"),
]).await?;

// Multi-turn (allows N rounds of tool calls)
let res = agent
    .prompt("Do X then Y")
    .multi_turn(2)
    .send()
    .await?;
```

## Prompt Hooks

Add custom behavior to the agent prompt loop by implementing `PromptHook`:

```rust
use rig::agent::PromptHook;

struct MyHook;

impl PromptHook for MyHook {
    // Override only the methods you need.
    // Available hooks: on_prompt, on_response, on_tool_call, on_tool_result
    // Use CancelSignal::cancel() to abort the loop early.
}
```

Prompt hooks are non-blocking. Keep computation light to avoid slowing the agent loop.

## Manual Tool Implementation

The `Tool` trait requires:
- `const NAME: &'static str` — unique identifier
- `type Args` — deserializable input (implement `Deserialize + JsonSchema`)
- `type Output` — serializable output
- `type Error` — error type
- `async fn definition(&self, _prompt: String) -> ToolDefinition`
- `async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>`

```rust
use rig::tool::{Tool, ToolDefinition};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, schemars::JsonSchema)]
struct AddArgs {
    /// First number to add
    x: i32,
    /// Second number to add
    y: i32,
}

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = anyhow::Error;
    type Args = AddArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let parameters = schemars::schema_for!(AddArgs);
        ToolDefinition {
            name: "add".to_string(),
            description: "Add x and y together".to_string(),
            parameters: serde_json::to_value(parameters).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.x + args.y)
    }
}
```

## `#[rig_tool]` Derive Macro

For simple tools without custom state or error types:

```rust
use rig_derive::tool_macro;

#[rig_tool(
    description = "Perform basic arithmetic operations",
    required(x, y, operation)
)]
fn calculator(x: i32, y: i32, operation: String) -> Result<i32, rig::tool::ToolError> {
    match operation.as_str() {
        "add" => Ok(x + y),
        "subtract" => Ok(x - y),
        "multiply" => Ok(x * y),
        "divide" => {
            if y == 0 {
                Err(rig::tool::ToolError::ToolCallError("Division by zero".into()))
            } else {
                Ok(x / y)
            }
        }
        _ => Err(rig::tool::ToolError::ToolCallError(
            format!("Unknown operation: {operation}").into(),
        )),
    }
}
```

Note: OpenAI Responses API requires all inputs to be listed in `required(...)`.

## RAG-Enabled Tools (ToolEmbedding)

Implement `ToolEmbedding` so tools can be stored in vector stores and retrieved semantically:

```rust
impl ToolEmbedding for Add {
    type InitError = anyhow::Error;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(Add)
    }

    fn embedding_docs(&self) -> Vec<String> {
        vec!["Add x and y together".into()]
    }

    fn context(&self) -> Self::Context {}
}
```

Then use with `.dynamic_tools(n, vector_store_index, toolset)` on the agent builder.

## Tool Servers

Avoid `Arc<Mutex<T>>` by running tools as Tokio-spawned message-passing tasks:

```rust
use rig::tool::server::{ToolServer, ToolServerHandle};

let tool_server: ToolServerHandle = ToolServer::new()
    .tool(Adder)
    .tool(Subtractor)
    .run();

// Clone the handle to share between multiple agents
let agent_a = openai.agent("gpt-4o").tool_server(tool_server.clone()).build();
let agent_b = openai.agent("gpt-4o").tool_server(tool_server.clone()).build();
```

Tool servers accept static tools, dynamic tools, and MCP tools. The server loop runs as long as at least one handle clone exists.

## schemars v1.0 Notes

Migration from v0.8:
- `#[schemars(description = "...")]` → use `///` doc comments on fields
- `schema_for!` macro → `schemars::schema_for!`
- Cargo.toml: `schemars = "1"` (not `"0.8"`)

Doc comments on struct fields become the JSON schema `description` automatically:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
struct MyArgs {
    /// The user's full name
    name: String,
    /// Age in years (must be positive)
    age: u32,
}
```
