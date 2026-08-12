// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! MCP server (`rmcp`) for `tvscreener-mcp` (stdio or Streamable HTTP).

use std::sync::Arc;

use rmcp::{
    handler::server::{tool::ToolCallContext, wrapper::Parameters},
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    tool, tool_router,
    transport::stdio,
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
};

use crate::mcp::tool_args::{
    AssetTypeArgs, BuildPayloadArgs, CustomQueryArgs, DiscoverFieldsArgs, GetPresetArgs,
    GetTopMoversArgs, SearchByIndexArgs, SearchCryptoArgs, SearchForexArgs, SearchStocksArgs,
};
use crate::mcp::tools;

/// MCP server handle exposing screener tools.
#[derive(Clone, Default)]
pub struct TvscreenerMcp;

/// Default HTTP bind address for Streamable MCP.
pub const DEFAULT_HTTP_LISTEN: &str = "0.0.0.0:6790";

fn text_ok(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text.into())])
}

fn text_err(err: impl std::fmt::Display) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(err.to_string())])
}

#[tool_router]
impl TvscreenerMcp {
    /// Search field catalog by keyword.
    #[tool(
        description = "Search available screener fields/indicators by keyword (use before custom_query)"
    )]
    async fn discover_fields(
        &self,
        Parameters(DiscoverFieldsArgs {
            search_term,
            asset_type,
            limit,
        }): Parameters<DiscoverFieldsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let asset = match tools::parse_asset(asset_type.as_deref().unwrap_or("stock")) {
            Ok(a) => a,
            Err(err) => return Ok(text_err(err)),
        };
        let limit = usize::try_from(limit.unwrap_or(20)).unwrap_or(20);
        Ok(text_ok(tools::format_discover_fields(
            asset,
            &search_term,
            limit,
        )))
    }

    /// List field category samples.
    #[tool(description = "List field categories with sample field names for an asset type")]
    async fn list_field_types(
        &self,
        Parameters(AssetTypeArgs { asset_type }): Parameters<AssetTypeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let asset = match tools::parse_asset(asset_type.as_deref().unwrap_or("stock")) {
            Ok(a) => a,
            Err(err) => return Ok(text_err(err)),
        };
        Ok(text_ok(tools::format_field_types(asset)))
    }

    /// Flexible query with fields and JSON filters.
    #[tool(description = "Flexible screener query: fields CSV, filters JSON array, sort_by, limit")]
    async fn custom_query(
        &self,
        Parameters(CustomQueryArgs {
            asset_type,
            fields,
            filters,
            sort_by,
            ascending,
            limit,
        }): Parameters<CustomQueryArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(
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
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
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
        Parameters(SearchStocksArgs {
            price_range,
            market_cap_billions_range,
            sectors,
            sort_by,
            limit,
        }): Parameters<SearchStocksArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (min_price, max_price) = tools::parse_f64_range(price_range.as_deref());
        let (min_mcap, max_mcap) = tools::parse_f64_range(market_cap_billions_range.as_deref());
        Ok(
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
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
    }

    /// Simplified crypto screen.
    #[tool(description = "Screen cryptocurrencies by 24h volume and market cap")]
    async fn search_crypto(
        &self,
        Parameters(SearchCryptoArgs {
            min_volume_millions,
            min_market_cap_billions,
            limit,
        }): Parameters<SearchCryptoArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(
            match tools::search_crypto(
                min_volume_millions,
                min_market_cap_billions,
                limit.unwrap_or(25),
            )
            .await
            {
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
    }

    /// Simplified forex screen.
    #[tool(description = "Screen forex currency pairs")]
    async fn search_forex(
        &self,
        Parameters(SearchForexArgs {
            min_volume_millions,
            limit,
        }): Parameters<SearchForexArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(
            match tools::search_forex(min_volume_millions, limit.unwrap_or(25)).await {
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
    }

    /// Top gainers or losers.
    #[tool(description = "Top gaining or losing stocks/crypto by change percent")]
    async fn get_top_movers(
        &self,
        Parameters(GetTopMoversArgs {
            asset_type,
            direction,
            limit,
        }): Parameters<GetTopMoversArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(
            match tools::get_top_movers(
                asset_type.as_deref().unwrap_or("stock"),
                direction.as_deref().unwrap_or("gainers"),
                limit.unwrap_or(10),
            )
            .await
            {
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
    }

    /// List preset names.
    #[tool(description = "List available field presets (crypto_price, stock_valuation, …)")]
    async fn list_presets(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_presets()))
    }

    /// Show one preset's fields.
    #[tool(description = "Show fields included in a named preset")]
    async fn get_preset(
        &self,
        Parameters(GetPresetArgs { name }): Parameters<GetPresetArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(match tools::format_get_preset(&name) {
            Ok(text) => text_ok(text),
            Err(err) => text_err(err),
        })
    }

    /// List stock sectors for filtering.
    #[tool(description = "List stock sectors (const → wire) for SECTOR filters / search_stocks")]
    async fn list_sectors(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_sectors()))
    }

    /// List countries.
    #[tool(description = "List countries (const → wire) for country filters")]
    async fn list_countries(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_countries()))
    }

    /// List industries.
    #[tool(description = "List industries (const → wire) for industry filters")]
    async fn list_industries(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_industries()))
    }

    /// List exchanges.
    #[tool(description = "List exchanges (const → wire) for exchange filters")]
    async fn list_exchanges(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_exchanges()))
    }

    /// List recommendation rating bands.
    #[tool(description = "List recommendation rating bands (STRONG_BUY … UNKNOWN)")]
    async fn list_ratings(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_ratings()))
    }

    /// List filter operators for `custom_query`.
    #[tool(description = "List filter operators accepted by custom_query (with JSON examples)")]
    async fn list_filter_operators(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_filter_operators()))
    }

    /// List stock markets.
    #[tool(description = "List stock markets (const → wire) for set_markets / build_payload")]
    async fn list_markets(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_markets()))
    }

    /// List index symbols for `set_index`.
    #[tool(
        description = "List index symbolsets (SP500, NASDAQ_100, …) for set_index / search_by_index"
    )]
    async fn list_index_symbols(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(tools::format_list_index_symbols()))
    }

    /// Dry-run scan payload JSON (no HTTP).
    #[tool(
        description = "Build scanner POST JSON without calling TradingView; optional stock indices/markets CSV"
    )]
    async fn build_payload(
        &self,
        Parameters(BuildPayloadArgs {
            asset_type,
            fields,
            filters,
            sort_by,
            indices,
            markets,
        }): Parameters<BuildPayloadArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(
            match tools::build_payload(&tools::BuildPayloadOpts {
                asset_type: asset_type.as_deref().unwrap_or("stock"),
                fields: fields.as_deref(),
                filters: filters.as_deref(),
                sort_by: sort_by.as_deref(),
                indices: indices.as_deref(),
                markets: markets.as_deref(),
            }) {
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
    }

    /// Live stock screen by index (+ optional markets).
    #[tool(
        description = "Screen stocks in index(es); indices CSV e.g. SP500,NASDAQ_100; optional markets CSV"
    )]
    async fn search_by_index(
        &self,
        Parameters(SearchByIndexArgs {
            indices,
            markets,
            fields,
            sort_by,
            limit,
        }): Parameters<SearchByIndexArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(
            match tools::search_by_index(
                &indices,
                markets.as_deref(),
                fields.as_deref(),
                sort_by.as_deref(),
                limit.unwrap_or(25),
            )
            .await
            {
                Ok(text) => text_ok(text),
                Err(err) => text_err(err),
            },
        )
    }
}

/// Serves MCP over stdio until the client disconnects.
///
/// # Errors
///
/// Returns transport / protocol errors from rmcp.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = TvscreenerMcp;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Serves MCP over Streamable HTTP until the process is stopped.
///
/// # Errors
///
/// Returns I/O errors from binding or serving the Axum listener.
pub async fn run_http(addr: &str) -> std::io::Result<()> {
    let config =
        rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig::default();
    let service = rmcp::transport::streamable_http_server::tower::StreamableHttpService::new(
        || Ok(TvscreenerMcp),
        Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        ),
        config,
    );
    let method_router = axum::routing::any_service(service);
    let app = axum::Router::new()
        .route("/mcp", method_router.clone())
        .route("/mcp/", method_router);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "tvscreener-mcp HTTP listening");
    axum::serve(listener, app).await?;
    Ok(())
}

impl ServerHandler for TvscreenerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "tvscreener-rs",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "TradingView screener MCP tools: discover fields, custom_query, search helpers, catalog lists, build_payload, search_by_index.",
            )
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::tool_router().get(name).cloned()
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let tcc = ToolCallContext::new(self, request, context);
        Self::tool_router().call(tcc).await
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(
            Self::tool_router().list_all(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_version_matches_crate() {
        let info = TvscreenerMcp.get_info();
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.server_info.name.as_str(), "tvscreener-rs");
    }
}
