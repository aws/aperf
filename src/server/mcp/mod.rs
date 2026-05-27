//! MCP (Model Context Protocol) server for APerf report analysis.

mod js_parser;
mod metadata;
mod report;
mod tools;

use anyhow::Result;
use rmcp::ServiceExt;

/// Entry point: start the MCP server on stdio.
pub fn run_mcp_server() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let service = tools::AperfMcpServer::new()
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;

        service.waiting().await?;
        Ok(())
    })
}
