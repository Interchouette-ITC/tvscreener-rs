// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! [`CoinScreener`] (`TradingView` subtype `coin`).
//!
//! Default coin fields, market-cap sort, and `price_conversion` misc.

use super::Screener;
use crate::field::{coin_market_cap, default_coin_fields};
use serde_json::json;

/// Coin screener with asset-class defaults
#[derive(Debug, Clone)]
pub struct CoinScreener {
    inner: Screener,
}

impl CoinScreener {
    /// Creates a screener with default coin fields and descending market-cap sort.
    pub fn new() -> Self {
        let mut inner = Screener::new("coin");
        inner.set_url(crate::util::get_url("coin"));
        inner.set_all_fields_fn(default_coin_fields);
        inner.select(default_coin_fields());
        inner.sort_by(coin_market_cap().field_name, false);
        inner.add_misc("price_conversion", json!({ "to_symbol": false }));
        Self { inner }
    }
}

impl_typed_screener_common!(CoinScreener);
