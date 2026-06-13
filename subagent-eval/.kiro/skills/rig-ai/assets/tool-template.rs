//! Template: Rig Tool Implementation
//!
//! Replace: ToolName, tool_name, ToolArgs, field descriptions, call logic.
//! Delete this header comment block when using.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Arguments for the tool. Doc comments become JSON schema descriptions
/// when using schemars::schema_for!().
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ToolNameArgs {
    /// Description of first parameter
    pub param_one: String,
    /// Description of second parameter
    pub param_two: i32,
}

/// Error type for this tool.
#[derive(Debug, thiserror::Error)]
#[error("ToolName error: {0}")]
pub struct ToolNameError(String);

#[derive(Deserialize, Serialize)]
pub struct ToolName;

impl Tool for ToolName {
    const NAME: &'static str = "tool_name";
    type Error = ToolNameError;
    type Args = ToolNameArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Brief description of what this tool does".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "param_one": {
                        "type": "string",
                        "description": "Description of first parameter"
                    },
                    "param_two": {
                        "type": "number",
                        "description": "Description of second parameter"
                    }
                },
                "required": ["param_one", "param_two"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // TODO: Implement tool logic
        Ok(format!("{} - {}", args.param_one, args.param_two))
    }
}
