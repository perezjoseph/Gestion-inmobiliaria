//! Template: Rig Agent Module
//!
//! A self-contained module that creates a configured agent.
//! Replace: module purpose, model name, preamble, tools.
//! Delete this header comment block when using.

use anyhow::Result;
use rig::completion::{Chat, Completion, Prompt};
use rig::providers::openai;

/// Create the configured agent.
/// Returns an agent that implements Prompt, Chat, and Completion traits.
pub fn create_agent() -> impl Prompt + Chat {
    let client = openai::Client::from_env();

    client
        .agent("gpt-4o")
        .preamble("You are a helpful assistant specialized in [DOMAIN].")
        .temperature(0.7)
        // .context("Static context always included in every request")
        // .tool(MyTool)
        // .dynamic_context(3, vector_index)
        .build()
}

/// One-shot prompt helper.
pub async fn ask(question: &str) -> Result<String> {
    let agent = create_agent();
    let response = agent.prompt(question).await?;
    Ok(response)
}

/// Chat with history helper.
pub async fn chat(message: &str, history: Vec<rig::completion::message::Message>) -> Result<String> {
    let agent = create_agent();
    let response = agent.chat(message, history).await?;
    Ok(response)
}
