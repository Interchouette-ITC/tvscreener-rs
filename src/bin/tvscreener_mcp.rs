// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener-mcp` - MCP server (stdio by default, optional Streamable HTTP).
//!
//! Logging (stderr): `RUST_LOG=tvscreener=debug` or `TVSCREENER_DEBUG=1`.
//!
//! ```bash
//! tvscreener-mcp
//! tvscreener-mcp --http
//! tvscreener-mcp --http --listen 0.0.0.0:8787
//! TVSCREENER_MCP_HTTP=1 tvscreener-mcp
//! ```

use anyhow::Result;
use clap::Parser;
use tvscreener::mcp::server::{run, run_http, DEFAULT_HTTP_LISTEN};

#[derive(Debug, Parser)]
#[command(
    name = "tvscreener-mcp",
    about = "TradingView screener MCP server (stdio or Streamable HTTP)",
    version
)]
struct Cli {
    /// Serve Streamable HTTP instead of stdio (also: `TVSCREENER_MCP_HTTP=1`).
    #[arg(long, env = "TVSCREENER_MCP_HTTP")]
    http: bool,

    /// HTTP bind address when `--http` is set (also: `TVSCREENER_MCP_ADDR`).
    #[arg(long, env = "TVSCREENER_MCP_ADDR", default_value = DEFAULT_HTTP_LISTEN)]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tvscreener::logging::init_logging();
    let cli = Cli::parse();

    if cli.http {
        tracing::info!(addr = %cli.listen, "tvscreener-mcp starting (HTTP)");
        run_http(&cli.listen)
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))?;
    } else {
        tracing::info!("tvscreener-mcp starting (stdio)");
        run().await.map_err(|err| anyhow::anyhow!("{err}"))?;
    }
    Ok(())
}
