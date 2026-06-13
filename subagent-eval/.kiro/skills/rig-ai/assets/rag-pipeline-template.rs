//! Template: RAG Pipeline
//!
//! Loads documents, embeds them, stores in a vector store, and creates
//! a RAG-enabled agent that retrieves context per query.
//! Replace: document source, embedding model, top-K, preamble.
//! Delete this header comment block when using.

use anyhow::Result;
use rig::client::{CompletionClient, EmbeddingsClient, ProviderClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingsBuilder;
use rig::loaders::FileLoader;
use rig::providers::openai;
use rig::vector_store::InMemoryVectorStore;

pub async fn build_rag_agent() -> Result<impl Prompt> {
    let client = openai::Client::from_env();
    let embed_model = client.embedding_model("text-embedding-3-small");

    // 1. Load documents
    let docs: Vec<String> = FileLoader::with_glob("docs/**/*.md")?
        .read()
        .ignore_errors()
        .collect();

    // 2. Generate embeddings
    let embeddings = EmbeddingsBuilder::new(embed_model.clone())
        .documents(docs)?
        .build()
        .await?;

    // 3. Store in vector store
    let store = InMemoryVectorStore::new();
    store.insert_documents(embeddings).await?;
    let index = store.index(embed_model);

    // 4. Create RAG agent
    let agent = client
        .agent("gpt-4o")
        .preamble("Answer questions based on the provided documentation. If the context doesn't contain the answer, say so.")
        .dynamic_context(3, index) // retrieve top-3 relevant docs per query
        .temperature(0.2)          // low temp for factual answers
        .build();

    Ok(agent)
}

pub async fn query(agent: &impl Prompt, question: &str) -> Result<String> {
    let response = agent.prompt(question).await?;
    Ok(response)
}
