// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener-mcp` - stdio MCP server (`cargo run --features mcp --bin tvscreener-mcp`).
//!
//! Logging (stderr): `RUST_LOG=tvscreener=debug` or `TVSCREENER_DEBUG=1`.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tvscreener::logging::init_logging();
    tracing::info!("tvscreener-mcp starting (stdio)");
    tvscreener::mcp::server::run()
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(())
}
