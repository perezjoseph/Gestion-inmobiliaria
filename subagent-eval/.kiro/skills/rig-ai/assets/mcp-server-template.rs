//! Template: MCP Server
//!
//! Exposes tools via Model Context Protocol so any MCP client can discover
//! and invoke them. Uses rmcp crate with streamable HTTP transport.
//! Replace: server name, tool implementations.
//! Delete this header comment block when using.
//!
//! Dependencies:
//!   cargo add rmcp -F server,macros,transport-streamable-http-server
//!   cargo add tokio -F full

use rmcp::prelude::*;

#[derive(Server)]
#[server(name = "my-service", version = "1.0.0")]
struct MyMcpServer;

#[server_impl]
impl MyMcpServer {
    /// Add two numbers together.
    #[tool(description = "Add two numbers together")]
    async fn add(&self, a: f64, b: f64) -> Result<f64> {
        Ok(a + b)
    }

    /// Multiply two numbers.
    #[tool(description = "Multiply two numbers")]
    async fn multiply(&self, a: f64, b: f64) -> Result<f64> {
        Ok(a * b)
    }

    // Add more tools here following the same pattern:
    // #[tool(description = "...")]
    // async fn tool_name(&self, param: Type) -> Result<ReturnType> { ... }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = MyMcpServer;

    let transport = rmcp::transport::StreamableHttpServerTransport::new(
        "127.0.0.1:8080".parse()?,
    );

    println!("MCP server running on http://127.0.0.1:8080");
    server.serve(transport).await?;

    Ok(())
}
