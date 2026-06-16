---
name: rig-ai
description: Build LLM-powered applications in Rust using the Rig framework (rig-core crate). Use when creating AI agents, implementing tool calling, building RAG systems, working with embeddings, streaming completions, structured data extraction, pipelines/chains, file loaders, evals, observability/tracing, image/audio generation, transcription, or integrating LLM providers (OpenAI, Anthropic, DeepSeek). Use when adding rig-core as a dependency, implementing the Tool trait, creating AgentBuilder pipelines, connecting to vector stores (Qdrant, MongoDB, LanceDB, Neo4j, SurrealDB), or using Model Context Protocol (MCP) with rmcp.
---

# Rig: Rust AI Agent Framework

Rig is a Rust library for building portable, modular, and lightweight fullstack AI agents. It provides ergonomic abstractions over LLM providers, tool calling, RAG, embeddings, streaming, structured extraction, pipelines, loaders, evals, and observability.

- Crate: `rig-core` (~0.37.x) | Runtime: Tokio
- Docs: https://docs.rig.rs | API: https://docs.rs/rig-core
- GitHub: https://github.com/0xPlaygrounds/rig

## Setup

```toml
[dependencies]
rig-core = "0.37"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1"  # MUST be v1, not v0.8 — breaking API differences
```

Optional feature flags: `rmcp` (MCP), `image`, `audio`, `experimental` (evals), `derive` (Embed macro).

## Decision Guide: Which Abstraction?

| I want to... | Use |
|---|---|
| Fire-and-forget prompt → String | `agent.prompt("...")` (`Prompt` trait) |
| Chat with history | `agent.chat("...", history)` (`Chat` trait) |
| Get typed/structured response | `agent.prompt_typed::<T>("...")` (`TypedPrompt` trait) |
| Extract structured data from text | `openai.extractor::<T>(model).build()` |
| Stream tokens incrementally | `agent.stream_prompt("...")` (`StreamingPrompt` trait) |
| Fine-grained request control | `model.completion_request("...").temperature(0.8).send()` |
| Compose multi-step AI workflows | `pipeline::new().map(...).prompt(model)` (`Op` trait) |
| Load files for context/RAG | `FileLoader::with_glob("*.txt")` or `PdfFileLoader` |
| Test LLM output quality | `LlmJudgeMetric`, `SemanticSimilarityMetric` (evals) |
| Generate images | `openai.image_generation_model("dall-e-3")` (requires `image` feature) |
| Text-to-speech | `openai.audio_generation_model("tts-1")` (requires `audio` feature) |
| Speech-to-text | `openai.transcription_model("whisper-1")` then `.transcription_request().load_file(path).send()` |
| Share tools across agents | `ToolServer::new().tool(T).run()` → `ToolServerHandle` |
| Use MCP tools from external server | `.rmcp_tools(tools, client.peer())` (requires `rmcp` feature) |

## Core Pattern: Provider → Agent → Prompt

```rust
use rig::providers::openai;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;

let openai = openai::Client::from_env(); // reads OPENAI_API_KEY

let agent = openai
    .agent("gpt-4o")
    .preamble("You are a helpful assistant.")
    .temperature(0.7)
    .tool(MyTool)                          // static tool (always available)
    .dynamic_context(3, vector_index)      // RAG: top-3 docs per query
    .dynamic_tools(2, tool_index, toolset) // RAG: top-2 tools per query
    .build();

let response = agent.prompt("Hello!").await?;
```

## Tools: When to Use What

- **Simple function** → `#[rig_tool]` macro (least boilerplate)
- **Need custom error types or state** → manual `impl Tool`
- **Tool should be RAG-retrievable** → also implement `ToolEmbedding`
- **Share tools across agents without Arc/Mutex** → `ToolServer`

For detailed API signatures and examples, read `references/tools-and-agents.md`.

## Key Gotchas

1. **schemars v1.0 required** — v0.8 uses `#[schemars(description = "...")]` which silently compiles but produces wrong schemas. v1.0 uses `///` doc comments on fields instead.
2. **Multi-turn defaults to 0** — if your agent has tools, call `.multi_turn(n)` or tool calls won't chain. Without it you get `MaxDepthError`.
3. **Provider clients are cheap to clone** — create once, share everywhere.
4. **`const NAME` must be unique** — two tools with the same name cause silent routing bugs.
5. **Evals require `experimental` feature** — `cargo add rig-core -F experimental`.
6. **Env vars**: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`.

## Templates

Copy from `assets/` when scaffolding new Rig code:

| Template | Use when... |
|----------|-------------|
| `assets/tool-template.rs` | Creating a new tool (impl Tool + Args struct) |
| `assets/agent-module-template.rs` | Setting up a new agent module with prompt/chat helpers |
| `assets/rag-pipeline-template.rs` | Building a RAG system (load → embed → store → agent) |
| `assets/mcp-server-template.rs` | Exposing tools via MCP protocol |
| `assets/extractor-template.rs` | Extracting structured data from unstructured text |

## Reference Files

For detailed code examples and API signatures beyond this guide, read:

- `references/tools-and-agents.md` — Tool trait, rig_tool macro, ToolEmbedding, ToolServer, AgentBuilder full API, Prompt Hooks
- `references/streaming-and-completions.md` — Completion traits, response types, Message/UserContent/AssistantContent enums, streaming patterns, PauseControl
- `references/rag-and-embeddings.md` — Embed trait, EmbeddingsBuilder, vector stores, FileLoader, PdfFileLoader, dynamic context
- `references/advanced.md` — Pipelines (Op trait, parallel!, TryOp, batch), Extractors, TypedPrompt, Evals (LlmJudge, Score, Similarity), Observability (Langfuse, OTel), Image/Audio/Transcription, MCP client/server
- `references/mcp-anti-patterns.md` — MCP server design anti-patterns, production best practices, review checklist, error handling, pagination, audit gates, idempotency, concurrency safety
