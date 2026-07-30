// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! MCP server (`mcpkit`) for `tvscreener-mcp` (stdio or Streamable HTTP).

#![allow(clippy::unused_async)]

use crate::mcp::tools;
use mcpkit::prelude::*;
use mcpkit::transport::stdio::StdioTransport;
use mcpkit_axum::McpRouter;

/// MCP server handle exposing screener tools.
pub struct TvscreenerMcp;

// mcpkit requires a string literal here; keep in sync with Cargo.toml `version`
// (enforced by `mcp_server_version_matches_crate` below).
#[mcp_server(name = "tvscreener-rs", version = "1.0.0")]
impl TvscreenerMcp {
    /// Search field catalog by keyword.
    #[tool(
        description = "Search available screener fields/indicators by keyword (use before custom_query)"
    )]
    async fn discover_fields(
        &self,
        search_term: String,
        asset_type: Option<String>,
        limit: Option<u32>,
    ) -> ToolOutput {
        let asset = match tools::parse_asset(asset_type.as_deref().unwrap_or("stock")) {
            Ok(a) => a,
            Err(err) => return ToolOutput::error(err.to_string()),
        };
        let limit = usize::try_from(limit.unwrap_or(20)).unwrap_or(20);
        ToolOutput::text(tools::format_discover_fields(asset, &search_term, limit))
    }

    /// List field category samples.
    #[tool(description = "List field categories with sample field names for an asset type")]
    async fn list_field_types(&self, asset_type: Option<String>) -> ToolOutput {
        let asset = match tools::parse_asset(asset_type.as_deref().unwrap_or("stock")) {
            Ok(a) => a,
            Err(err) => return ToolOutput::error(err.to_string()),
        };
        ToolOutput::text(tools::format_field_types(asset))
    }

    /// Flexible query with fields and JSON filters.
    #[tool(description = "Flexible screener query: fields CSV, filters JSON array, sort_by, limit")]
    async fn custom_query(
        &self,
        asset_type: Option<String>,
        fields: Option<String>,
        filters: Option<String>,
        sort_by: Option<String>,
        ascending: Option<bool>,
        limit: Option<u32>,
    ) -> ToolOutput {
        match tools::custom_query(&tools::CustomQueryOpts {
            asset_type: asset_type.as_deref().unwrap_or("stock"),
            fields: fields.as_deref(),
            filters: filters.as_deref(),
            sort_by: sort_by.as_deref(),
            ascending: ascending.unwrap_or(false),
            limit: limit.unwrap_or(25),
        })
        .await
        {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// Simplified stock screen.
    ///
    /// `price_range` / `market_cap_billions_range` are optional `"min,max"` strings
    /// (either side may be empty), e.g. `"10,"`, `",500"`, `"10,100"`.
    #[tool(
        description = "Screen stocks; optional price_range and market_cap_billions_range as min,max"
    )]
    async fn search_stocks(
        &self,
        price_range: Option<String>,
        market_cap_billions_range: Option<String>,
        sectors: Option<String>,
        sort_by: Option<String>,
        limit: Option<u32>,
    ) -> ToolOutput {
        let (min_price, max_price) = tools::parse_f64_range(price_range.as_deref());
        let (min_mcap, max_mcap) = tools::parse_f64_range(market_cap_billions_range.as_deref());
        match tools::search_stocks(&tools::SearchStocksOpts {
            min_price,
            max_price,
            min_market_cap_billions: min_mcap,
            max_market_cap_billions: max_mcap,
            sectors: sectors.as_deref(),
            sort_by: sort_by.as_deref().unwrap_or("market_cap"),
            limit: limit.unwrap_or(25),
        })
        .await
        {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// Simplified crypto screen.
    #[tool(description = "Screen cryptocurrencies by 24h volume and market cap")]
    async fn search_crypto(
        &self,
        min_volume_millions: Option<f64>,
        min_market_cap_billions: Option<f64>,
        limit: Option<u32>,
    ) -> ToolOutput {
        match tools::search_crypto(
            min_volume_millions,
            min_market_cap_billions,
            limit.unwrap_or(25),
        )
        .await
        {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// Simplified forex screen.
    #[tool(description = "Screen forex currency pairs")]
    async fn search_forex(
        &self,
        min_volume_millions: Option<f64>,
        limit: Option<u32>,
    ) -> ToolOutput {
        match tools::search_forex(min_volume_millions, limit.unwrap_or(25)).await {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// Top gainers or losers.
    #[tool(description = "Top gaining or losing stocks/crypto by change percent")]
    async fn get_top_movers(
        &self,
        asset_type: Option<String>,
        direction: Option<String>,
        limit: Option<u32>,
    ) -> ToolOutput {
        match tools::get_top_movers(
            asset_type.as_deref().unwrap_or("stock"),
            direction.as_deref().unwrap_or("gainers"),
            limit.unwrap_or(10),
        )
        .await
        {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// List preset names.
    #[tool(description = "List available field presets (crypto_price, stock_valuation, …)")]
    async fn list_presets(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_presets())
    }

    /// Show one preset's fields.
    #[tool(description = "Show fields included in a named preset")]
    async fn get_preset(&self, name: String) -> ToolOutput {
        match tools::format_get_preset(&name) {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// List stock sectors for filtering.
    #[tool(description = "List stock sectors (const → wire) for SECTOR filters / search_stocks")]
    async fn list_sectors(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_sectors())
    }

    /// List countries.
    #[tool(description = "List countries (const → wire) for country filters")]
    async fn list_countries(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_countries())
    }

    /// List industries.
    #[tool(description = "List industries (const → wire) for industry filters")]
    async fn list_industries(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_industries())
    }

    /// List exchanges.
    #[tool(description = "List exchanges (const → wire) for exchange filters")]
    async fn list_exchanges(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_exchanges())
    }

    /// List recommendation rating bands.
    #[tool(description = "List recommendation rating bands (STRONG_BUY … UNKNOWN)")]
    async fn list_ratings(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_ratings())
    }

    /// List filter operators for `custom_query`.
    #[tool(description = "List filter operators accepted by custom_query (with JSON examples)")]
    async fn list_filter_operators(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_filter_operators())
    }

    /// List stock markets.
    #[tool(description = "List stock markets (const → wire) for set_markets / build_payload")]
    async fn list_markets(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_markets())
    }

    /// List index symbols for `set_index`.
    #[tool(
        description = "List index symbolsets (SP500, NASDAQ_100, …) for set_index / search_by_index"
    )]
    async fn list_index_symbols(&self) -> ToolOutput {
        ToolOutput::text(tools::format_list_index_symbols())
    }

    /// Dry-run scan payload JSON (no HTTP).
    #[tool(
        description = "Build scanner POST JSON without calling TradingView; optional stock indices/markets CSV"
    )]
    async fn build_payload(
        &self,
        asset_type: Option<String>,
        fields: Option<String>,
        filters: Option<String>,
        sort_by: Option<String>,
        indices: Option<String>,
        markets: Option<String>,
    ) -> ToolOutput {
        match tools::build_payload(&tools::BuildPayloadOpts {
            asset_type: asset_type.as_deref().unwrap_or("stock"),
            fields: fields.as_deref(),
            filters: filters.as_deref(),
            sort_by: sort_by.as_deref(),
            indices: indices.as_deref(),
            markets: markets.as_deref(),
        }) {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }

    /// Live stock screen by index (+ optional markets).
    #[tool(
        description = "Screen stocks in index(es); indices CSV e.g. SP500,NASDAQ_100; optional markets CSV"
    )]
    async fn search_by_index(
        &self,
        indices: String,
        markets: Option<String>,
        fields: Option<String>,
        sort_by: Option<String>,
        limit: Option<u32>,
    ) -> ToolOutput {
        match tools::search_by_index(
            &indices,
            markets.as_deref(),
            fields.as_deref(),
            sort_by.as_deref(),
            limit.unwrap_or(25),
        )
        .await
        {
            Ok(text) => ToolOutput::text(text),
            Err(err) => ToolOutput::error(err.to_string()),
        }
    }
}

/// Serves MCP over stdio until the client disconnects.
///
/// # Errors
///
/// Returns transport / protocol errors from mcpkit.
pub async fn run() -> Result<(), McpError> {
    let transport = StdioTransport::new();
    let server = ServerBuilder::new(TvscreenerMcp)
        .with_tools(TvscreenerMcp)
        .build();
    server.serve(transport).await
}

/// Default HTTP bind address for Streamable MCP (`mcpkit-axum`).
pub const DEFAULT_HTTP_LISTEN: &str = "0.0.0.0:8787";

/// Serves MCP over Streamable HTTP until the process is stopped.
///
/// # Errors
///
/// Returns I/O errors from binding or serving the Axum listener.
pub async fn run_http(addr: &str) -> std::io::Result<()> {
    McpRouter::new(TvscreenerMcp).serve(addr).await
}

impl ResourceHandler for TvscreenerMcp {
    async fn list_resources(&self, _ctx: &Context<'_>) -> Result<Vec<Resource>, McpError> {
        Ok(Vec::new())
    }

    async fn read_resource(
        &self,
        uri: &str,
        _ctx: &Context<'_>,
    ) -> Result<Vec<ResourceContents>, McpError> {
        Err(McpError::invalid_params(
            "resources/read",
            format!("unknown resource: {uri}"),
        ))
    }
}

impl PromptHandler for TvscreenerMcp {
    async fn list_prompts(&self, _ctx: &Context<'_>) -> Result<Vec<Prompt>, McpError> {
        Ok(Vec::new())
    }

    async fn get_prompt(
        &self,
        name: &str,
        _args: Option<serde_json::Map<String, serde_json::Value>>,
        _ctx: &Context<'_>,
    ) -> Result<GetPromptResult, McpError> {
        Err(McpError::invalid_params(
            "prompts/get",
            format!("unknown prompt: {name}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mcp_server_version_matches_crate() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            "1.0.0",
            "bump #[mcp_server(version = …)] when changing Cargo.toml version"
        );
    }
}
