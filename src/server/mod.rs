//! Server module for APerf — exposes report analysis over various protocols.
//!
//! Currently supports MCP (Model Context Protocol) over stdio.

pub mod mcp;

use anyhow::Result;
use clap::Args;

/// CLI arguments for the `aperf server` subcommand.
#[derive(Args, Debug)]
pub struct Server {
    /// Start an MCP (Model Context Protocol) server over stdio.
    #[clap(long)]
    pub mcp: bool,
}

/// Run the server based on CLI flags.
pub fn run_server(args: &Server) -> Result<()> {
    if args.mcp {
        return mcp::run_mcp_server();
    }

    anyhow::bail!("No server type specified. Use --mcp to start an MCP server.")
}
