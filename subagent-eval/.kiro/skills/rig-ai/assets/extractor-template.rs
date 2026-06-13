//! Template: Rig Extractor (Structured Data Extraction)
//!
//! Extracts typed data from unstructured text using an LLM.
//! Replace: target struct, field descriptions, model, preamble.
//! Delete this header comment block when using.

use anyhow::Result;
use rig::providers::openai;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The structured data to extract. Use Option<T> for fields that may be absent.
/// Doc comments on fields become schema descriptions that guide the LLM.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ExtractedData {
    /// The entity's name
    pub name: Option<String>,
    /// A numerical value associated with the entity
    pub value: Option<f64>,
    /// Category or classification
    pub category: Option<String>,
}

pub async fn extract_from_text(text: &str) -> Result<ExtractedData> {
    let client = openai::Client::from_env();

    let extractor = client
        .extractor::<ExtractedData>("gpt-4o")
        .preamble("Extract structured data from the provided text. Use null for any field not clearly stated.")
        // .context("Additional domain context to help the LLM")
        .build();

    let result = extractor.extract(text).await?;
    Ok(result)
}

