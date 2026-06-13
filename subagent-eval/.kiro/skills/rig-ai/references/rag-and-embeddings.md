# RAG and Embeddings — Detailed Reference

## Embedding Models

Create an embedding model from a provider client:

```rust
use rig::client::{EmbeddingsClient, ProviderClient};
use rig::providers::openai;

let openai = openai::Client::from_env();
let model = openai.embedding_model("text-embedding-3-small");
```

## EmbeddingsBuilder

Batch-embed multiple documents:

```rust
use rig::embeddings::EmbeddingsBuilder;

let embeddings = EmbeddingsBuilder::new(model)
    .document("First document text")?
    .document("Second document text")?
    .build()
    .await?;
// Returns iterator over (T, OneOrMany<Embedding>)
```

For custom types:

```rust
let embeddings = EmbeddingsBuilder::new(model)
    .documents(vec![my_struct_1, my_struct_2])?
    .build()
    .await?;
```

## The Embed Trait

Types must implement `Embed` to be embeddable. Two approaches:

### Derive macro (requires `derive` feature)

```rust
use rig::Embed;

#[derive(Embed)]
struct Article {
    id: i32,
    #[embed]        // This field gets embedded
    title: String,
    #[embed]        // Multiple fields can be embedded
    body: String,
    author: String, // Not embedded (no #[embed] attribute)
}
```

### Manual implementation

```rust
use rig::embeddings::{Embed, TextEmbedder, EmbedError};

impl Embed for WordDefinition {
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        // Each call to embedder.embed() generates a separate embedding vector
        embedder.embed(self.word.clone());
        for def in &self.definitions {
            embedder.embed(def.clone());
        }
        Ok(())
    }
}
```

## Vector Stores

### In-Memory (built-in)

```rust
use rig::vector_store::InMemoryVectorStore;

let store = InMemoryVectorStore::new();
let index = store.index(embedding_model);
```

### Inserting documents

```rust
use rig::vector_store::InsertDocuments;

let embeddings = EmbeddingsBuilder::new(model)
    .documents(documents)?
    .build()
    .await?;

vector_store.insert_documents(embeddings).await?;
```

### External vector stores

| Store | Crate | Install |
|-------|-------|---------|
| MongoDB | `rig-mongodb` | `cargo add rig-mongodb` |
| Qdrant | `rig-qdrant` | `cargo add rig-qdrant` |
| LanceDB | `rig-lancedb` | `cargo add rig-lancedb` |
| Neo4j | `rig-neo4j` | `cargo add rig-neo4j` |
| SurrealDB | `rig-surrealdb` | `cargo add rig-surrealdb` |

## Dynamic Context (RAG Agent)

Attach a vector store index to an agent so it retrieves relevant documents per query:

```rust
let agent = openai
    .agent("gpt-4o")
    .preamble("Answer questions using the provided context.")
    .dynamic_context(5, document_index) // retrieve top-5 docs
    .temperature(0.3) // lower temp for factual answers
    .build();
```

Documents are automatically appended to the completion request before sending.

## File Loaders

### FileLoader (text files)

```rust
use rig::loaders::FileLoader;

// Glob pattern — recursive
let files = FileLoader::with_glob("**/*.txt")?
    .read()
    .ignore_errors()
    .into_iter();

// Directory
let files = FileLoader::with_dir("data/")?
    .read_with_path()
    .ignore_errors();

// From raw bytes (e.g., downloaded file)
let loader = FileLoader::from_bytes(bytes);
```

### PdfFileLoader

```rust
use rig::loaders::PdfFileLoader;

let pages = PdfFileLoader::with_glob("docs/*.pdf")?
    .load_with_path()
    .ignore_errors()
    .by_page()       // iterate page by page
    .into_iter();
```

### EpubFileLoader

```rust
use rig::loaders::EpubFileLoader;
// Similar API to PdfFileLoader
```

### Loading files into agent context

```rust
let examples = FileLoader::with_glob("examples/*.rs")?
    .read_with_path()
    .ignore_errors()
    .into_iter();

let agent = examples
    .fold(AgentBuilder::new(model), |builder, (path, content)| {
        builder.context(format!("Example {:?}:\n{}", path, content).as_str())
    })
    .build();
```

### Loading files into a vector store

```rust
let documents: Vec<String> = FileLoader::with_glob("knowledge/*.md")?
    .read()
    .ignore_errors()
    .collect();

let embeddings = EmbeddingsBuilder::new(embedding_model)
    .documents(documents)?
    .build()
    .await?;

vector_store.insert_documents(embeddings).await?;
```

## Full RAG Pipeline Example

```rust
use rig::providers::openai;
use rig::client::{CompletionClient, EmbeddingsClient, ProviderClient};
use rig::embeddings::EmbeddingsBuilder;
use rig::vector_store::InMemoryVectorStore;
use rig::loaders::FileLoader;
use rig::completion::Prompt;

let openai = openai::Client::from_env();
let embed_model = openai.embedding_model("text-embedding-3-small");

// 1. Load and embed documents
let docs: Vec<String> = FileLoader::with_glob("docs/*.md")?
    .read()
    .ignore_errors()
    .collect();

let embeddings = EmbeddingsBuilder::new(embed_model.clone())
    .documents(docs)?
    .build()
    .await?;

// 2. Store in vector store
let store = InMemoryVectorStore::new();
store.insert_documents(embeddings).await?;
let index = store.index(embed_model);

// 3. Create RAG agent
let agent = openai
    .agent("gpt-4o")
    .preamble("Answer questions based on the provided documentation.")
    .dynamic_context(3, index)
    .build();

// 4. Query
let answer = agent.prompt("How do I configure authentication?").await?;
```

## Best Practices

- **Chunk large documents** before embedding — most embedding models have token limits (~8K tokens). Split by paragraph or section.
- **Clean text** before embedding — remove boilerplate, navigation, headers that don't carry semantic meaning.
- **Use `ignore_errors()`** for fault-tolerant batch processing. Handle errors individually when precision matters.
- **Lower temperature** for RAG agents (0.1–0.3) since you want factual answers grounded in context.
- **Monitor embedding dimensions** — different models produce different vector sizes. Ensure your vector store is configured for the right dimensionality.
