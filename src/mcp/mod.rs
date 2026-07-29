// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! MCP tool helpers and mcpkit stdio server (`feature = "mcp"`).
//!
//! Tools:
//!
//! - `discover_fields`, `list_field_types`
//! - `custom_query`
//! - `search_stocks` / `search_crypto` / `search_forex`
//! - `get_top_movers`
//! - `list_presets` / `get_preset`
//! - `list_sectors` / `list_countries` / `list_industries` / `list_exchanges` / `list_ratings`
//! - `list_filter_operators`
//! - `list_markets` / `list_index_symbols`
//! - `build_payload` / `search_by_index`

pub mod format;
pub mod query;
pub mod resolve;
pub mod server;
pub mod tools;
