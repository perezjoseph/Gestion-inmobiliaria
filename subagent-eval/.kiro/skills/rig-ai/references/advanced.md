# Advanced Features — Detailed Reference

## Pipelines (Chains)

Composable processing pipelines inspired by Airflow/Dagster. Built on the `Op` trait.

### Basic pipeline

```rust
use rig::pipeline::{self, Op};

let pipeline = pipeline::new()
    .map(|(x, y)| x + y)     // sync transform
    .map(|z| z * 2)
    .map(|n| n.to_string());

let result = pipeline.call((5, 3)).await;
assert_eq!(result, "16");
```

### Async operations

```rust
let pipeline = pipeline::new()
    .then(|x| async move { fetch_data(x).await });
```

### Parallel operations

```rust
use rig::pipeline::parallel;

let pipeline = pipeline::new()
    .chain(parallel!(
        passthrough(),                              // pass input through unchanged
        lookup::<_, _, Document>(vector_store, 3)   // retrieve 3 docs
    ))
    .map(|(query, docs)| format!("Query: {}\nContext: {}", query, docs.join("\n")))
    .prompt(llm_model);
```

### Custom Op

```rust
struct Tokenizer;

impl Op for Tokenizer {
    type Input = String;
    type Output = Vec<String>;

    async fn call(&self, input: Self::Input) -> Self::Output {
        input.split_whitespace().map(String::from).collect()
    }
}
```

### TryOp (fallible operations)

For operations that can fail:

```rust
let result = op.try_batch_call(2, vec![2, 4]).await;
assert_eq!(result, Ok(vec![3, 5]));
```

### Batch processing

Process multiple inputs concurrently:

```rust
let results = pipeline.batch_call(5, documents).await; // 5 concurrent
```

### RAG pipeline example

```rust
let rag_pipeline = pipeline::new()
    .chain(parallel!(
        passthrough(),
        lookup::<_, _, Document>(vector_store, 3)
    ))
    .map(|(query, docs)| format!(
        "Query: {}\nContext: {}",
        query,
        docs.join("\n")
    ))
    .prompt(llm_model);

let answer = rag_pipeline.call("What is Rig?").await;
```

---

## Extractors

Extract structured data from unstructured text. Uses an internal "submit" tool that the LLM calls with the extracted data.

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema)]
struct Person {
    /// The person's full name
    name: Option<String>,
    /// Age in years
    age: Option<u8>,
    /// Their job title or profession
    profession: Option<String>,
}

// Basic extractor
let extractor = openai.extractor::<Person>("gpt-4o").build();
let person = extractor.extract("John Doe is a 30 year old doctor.").await?;

// With customization
let extractor = openai
    .extractor::<Person>("gpt-4o")
    .preamble("Extract person details with high precision. Use None for missing fields.")
    .context("Names follow Western convention: first name then last name.")
    .build();
```

Requirements for target type: `Deserialize + Serialize + JsonSchema`

Use `Option<T>` for fields that may not be present in the text.

### Error handling

```rust
pub enum ExtractionError {
    NoData,                          // Model didn't call the submit tool
    DeserializationError(serde_json::Error),
    PromptError(PromptError),
}
```

If you get `NoData` repeatedly, the model may be too weak. Try a stronger model (gpt-4o over gpt-3.5).

### Batch extraction

```rust
async fn extract_all(extractor: &Extractor<Model, Person>, texts: Vec<String>) -> Vec<Result<Person, ExtractionError>> {
    let mut results = Vec::new();
    for text in texts {
        results.push(extractor.extract(&text).await);
    }
    results
}
```

---

## Structured Output (TypedPrompt)

Different from Extractors: TypedPrompt asks the agent to respond in a structured format directly, without the submit-tool pattern.

```rust
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
struct SentimentAnalysis {
    /// Sentiment score from -1.0 (negative) to 1.0 (positive)
    score: f64,
    /// One of: positive, negative, neutral
    label: String,
    /// Brief explanation of the sentiment
    reasoning: String,
}

let result: SentimentAnalysis = agent
    .prompt_typed("Analyze: 'This product exceeded my expectations!'")
    .await?;
```

Error type: `StructuredOutputError` (wraps `CompletionError` or `JsonError`).

---

## Evals Framework

> Requires: `cargo add rig-core -F experimental`

### Core trait

```rust
use rig::evals::{Eval, EvalOutcome, EvalError};

pub trait Eval {
    type Input;
    type Output;

    async fn eval(&self, input: Self::Input, output: Self::Output) -> Result<EvalOutcome, EvalError>;
}

pub enum EvalOutcome {
    Pass,
    Fail,
    Invalid(String), // eval itself failed (e.g., unparseable judge response)
}
```

### LLM Judge

Uses an LLM to judge output quality. Define a judgment schema:

```rust
use rig::evals::{LlmJudgeMetric, Judgment};

#[derive(Deserialize, JsonSchema)]
struct FactualityJudgment {
    /// Whether the response is factually accurate
    is_factual: bool,
    /// Explanation for the judgment
    reasoning: String,
}

impl Judgment for FactualityJudgment {
    fn passed(&self) -> bool { self.is_factual }
}

let judge = LlmJudgeMetric::<FactualityJudgment>::builder(model)
    .preamble("You are a factuality judge. Evaluate whether the response is accurate.")
    .build();

let outcome = judge.eval(
    "What is the capital of France?",
    "The capital of France is Paris."
).await?;
```

### LLM Judge with custom function

```rust
let judge = LlmJudgeMetric::<MySchema>::builder(model)
    .preamble("Evaluate the response.")
    .with_judge_fn(|schema: &MySchema| schema.score > 0.5)
    .build();
```

### LLM Score

Assigns a numerical score:

```rust
use rig::evals::LlmScoreMetric;

let scorer = LlmScoreMetric::builder(model)
    .preamble("Score the response quality from 0 to 10.")
    .threshold(7.0) // scores >= 7.0 pass
    .build();

let outcome = scorer.eval("Explain X", "X is...").await?;
```

### Semantic Similarity

Non-LLM metric using cosine similarity between embeddings:

```rust
use rig::evals::SemanticSimilarityMetric;

let metric = SemanticSimilarityMetric::builder(embedding_model)
    .threshold(0.85) // cosine similarity >= 0.85 passes
    .build();

let outcome = metric.eval(
    "The cat sat on the mat",     // expected
    "A cat was sitting on a mat"  // actual
).await?;
```

### Custom eval

```rust
struct ContainsKeyword { keyword: String }

impl Eval for ContainsKeyword {
    type Input = String;
    type Output = String;

    async fn eval(&self, _input: Self::Input, output: Self::Output) -> Result<EvalOutcome, EvalError> {
        if output.contains(&self.keyword) {
            Ok(EvalOutcome::Pass)
        } else {
            Ok(EvalOutcome::Fail)
        }
    }
}
```

### Best practices for evals

- Combine multiple metrics (factuality + relevance + length)
- LLM evals are non-deterministic — run multiple times for reliable results
- Start with permissive thresholds, tighten as you understand behavior
- Use cheaper models for judging when possible
- Always handle `EvalOutcome::Invalid`

---

## Observability

Rig uses the `tracing` ecosystem and follows OpenTelemetry GenAI Semantic Conventions.

### Langfuse integration (recommended, no collector needed)

```toml
[dependencies]
opentelemetry = "0.31"
opentelemetry_langfuse = "0.6"
tracing-opentelemetry = "0.31"
tracing-subscriber = "0.3"
```

```rust
use opentelemetry_langfuse::LangfuseTracer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let tracer = LangfuseTracer::builder()
        .with_public_key(std::env::var("LANGFUSE_PUBLIC_KEY").unwrap())
        .with_secret_key(std::env::var("LANGFUSE_SECRET_KEY").unwrap())
        .with_host("https://cloud.langfuse.com")
        .build()
        .expect("failed to create Langfuse tracer");

    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

Once initialized, model calls, agent invocations, multi-turn loops, and tool executions appear automatically in Langfuse.

### Attributes emitted

- `gen_ai.agent.name`
- `gen_ai.operation.name`

### Compatible backends

Langfuse, Arize Phoenix, any OpenTelemetry-compatible backend.

### Logs only (no spans)

If you don't want full tracing, use a simple `tracing_subscriber` with `fmt::layer()`:

```rust
tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new("info"))
    .with(tracing_subscriber::fmt::layer())
    .init();
```

---

## Image Generation

> Requires: `cargo add rig-core -F image`

```rust
let dalle = openai.image_generation_model("dall-e-3");

let response = dalle
    .image_generation_request("A futuristic city at sunset")
    .size("1024x1024")
    .send()
    .await?;

let image_data: Vec<u8> = response.image;
let raw = response.raw_response; // provider-specific JSON
```

## Audio Generation (TTS)

> Requires: `cargo add rig-core -F audio`

```rust
let tts = openai.audio_generation_model("tts-1");

let response = tts
    .audio_generation_request("Hello, how can I help you today?")
    .voice("alloy")
    .send()
    .await?;

let audio_bytes: Vec<u8> = response.audio;
```

## Transcription (STT)

The transcription API uses a builder pattern. You can either load a file from disk
or pass raw bytes directly:

```rust
use rig::providers::openai;

let openai = openai::Client::from_env();
let whisper = openai.transcription_model("whisper-1");

// Option A: Load file from path
let response = whisper
    .transcription_request()
    .load_file("audio.mp3")       // reads file into data + sets filename
    .language("en".to_string())
    .send()
    .await?;

println!("Transcription: {}", response.text);

// Option B: Pass raw bytes
let audio_bytes: Vec<u8> = std::fs::read("audio.mp3")?;
let response = whisper
    .transcription_request()
    .data(audio_bytes)             // set raw audio bytes
    .filename(Some("audio.mp3".to_string()))
    .language("en".to_string())
    .temperature(0.0)              // optional: lower = more deterministic
    .send()
    .await?;

println!("Transcription: {}", response.text);
```

Key points:
- `transcription_request()` takes no arguments — returns a builder
- Use `.load_file(path)` for convenience (reads + sets filename)
- Use `.data(Vec<u8>)` + `.filename(Some(...))` for raw bytes
- `.language()` takes a `String` (not `&str`)
- `.build()` panics if data is empty — prefer `.send()` which builds and sends
- Response type: `TranscriptionResponse { text: String, raw_response: T }`

---

## Model Context Protocol (MCP)

### Dependencies

```bash
cargo add rig-core -F rmcp
cargo add rmcp -F client,macros,transport-streamable-http-client-reqwest,transport-streamable-http-server
```

### Client: consuming tools from an MCP server

```rust
use rmcp::prelude::*;

let transport = rmcp::transport::StreamableHttpClientTransport::from_uri("http://localhost:8080");

let client_info = ClientInfo {
    protocol_version: Default::default(),
    capabilities: ClientCapabilities::default(),
    client_info: Implementation {
        name: "my-app".to_string(),
        version: "1.0.0".to_string(),
    },
};

let client = client_info.serve(transport).await?;
let tools: Vec<Tool> = client.list_tools(Default::default()).await?.tools;

// Pass MCP tools to a Rig agent
let agent = openai
    .agent("gpt-4o")
    .rmcp_tools(tools, client.peer().to_owned())
    .build();

let response = agent.prompt("Add 10 + 10").await?;
```

### Server: exposing tools via MCP

```rust
use rmcp::prelude::*;

#[derive(Server)]
#[server(name = "my-calculator-server", version = "1.0.0")]
struct CalculatorServer;

#[server_impl]
impl CalculatorServer {
    #[tool(description = "Add two numbers together")]
    async fn add(&self, a: f64, b: f64) -> Result<f64> {
        Ok(a + b)
    }

    #[tool(description = "Multiply two numbers")]
    async fn multiply(&self, a: f64, b: f64) -> Result<f64> {
        Ok(a * b)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = CalculatorServer;
    let transport = rmcp::transport::StreamableHttpServerTransport::new(
        "127.0.0.1:8080".parse()?
    );
    server.serve(transport).await?;
    Ok(())
}
```

The server handles tool discovery, capability negotiation, and invocations automatically per the MCP protocol.
