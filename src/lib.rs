// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Unofficial `TradingView` screener API client (Rust), inspired by
//! [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener).
//!
//! Canonical repository: [Interchouette-ITC/tvscreener-rs](https://github.com/Interchouette-ITC/tvscreener-rs).
//!
//! Builds JSON payloads and POSTs them to
//! `https://scanner.tradingview.com/{subtype}/scan`.
//!
//! # Payload shape
//!
//! [`core::Screener::build_payload`] emits a JSON object with `filter`, `columns`,
//! `range`, `options`, `symbols`, optional `sort`, and optional top-level keys from
//! [`core::Screener::add_misc`]. Filters with the same `left` key are merged: values
//! accumulate and the operator may promote to `in_range`.
//!
//! # Rows
//!
//! Scan responses decode to [`ScreenerRow`]: each row holds a symbol string and a
//! `data` map keyed by **column label** (not technical field name). Labels come from
//! the `(technical, label)` pairs produced by [`util::get_columns_to_request`].
//!
//! # Example (offline payload)
//!
//! ```
//! use tvscreener::core::Screener;
//! use tvscreener::field::default_crypto_fields;
//! use tvscreener::filter::{FieldCondition, FilterOperator};
//! use serde_json::json;
//!
//! let mut screener = Screener::new("crypto");
//! screener
//!.select(default_crypto_fields())
//!.where_condition(FieldCondition::new(
//! "name",
//! FilterOperator::Match,
//! json!("btc"),
//! )).unwrap()
//!.set_range(0, 5);
//! let payload = screener.build_payload().unwrap();
//! assert!(payload.get("columns").and_then(|v| v.as_array()).is_some());
//! ```
//!
//! Not affiliated with [TradingView](https://www.tradingview.com).
//! Use is subject to `TradingView` terms.

#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod beautify;
pub mod core;
pub mod error;
pub mod field;
pub mod filter;
pub mod logging;
pub mod mcp;
pub mod resolve;
pub mod ta;
pub mod util;

pub use beautify::{format_cell, format_cell_for_field, CellTone, FormattedCell, RowTechMap};
pub use error::{Result, TvscreenerError};
pub use field::{
    catalog_len, get_preset, list_fields, list_presets, resolve_field, search_fields, Asset,
    FieldDef, IndexSymbolDef, Market, NamedValue, RatingBand,
};
pub use filter::{ExtraFilter, FieldCondition, Filter, FilterOperator};
pub use util::{format_recommendation, format_row, format_value};

/// One row from a screener scan response.
///
/// Holds a symbol plus a map of column **label** → JSON value. Labels align with the
/// second element of each `(technical_name, label)` pair from
/// [`util::get_columns_to_request`], not wire keys.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)] // `serde_json::Value` is not `Eq`
pub struct ScreenerRow {
    /// `TradingView` symbol (e.g. `"BINANCE:BTCUSDT"`).
    pub symbol: String,
    /// Column label → value for the selected fields.
    pub data: serde_json::Map<String, serde_json::Value>,
}
