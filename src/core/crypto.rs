// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! [`CryptoScreener`] (`TradingView` subtype `crypto`).
//!
//! Default crypto fields, sort, and `price_conversion` misc.

use super::Screener;
use crate::field::{crypto_volume_24h_in_usd, default_crypto_fields};
use serde_json::json;

/// Crypto screener with asset-class defaults
#[derive(Debug, Clone)]
pub struct CryptoScreener {
    inner: Screener,
}

impl CryptoScreener {
    /// Creates a screener with default crypto fields and sort by 24h USD volume.
    pub fn new() -> Self {
        let mut inner = Screener::new("crypto");
        inner.set_url(crate::util::get_url("crypto"));
        inner.set_all_fields_fn(default_crypto_fields);
        inner.select(default_crypto_fields());
        inner.sort_by(crypto_volume_24h_in_usd().field_name, false);
        inner.add_misc("price_conversion", json!({ "to_symbol": false }));
        Self { inner }
    }
}

impl_typed_screener_common!(CryptoScreener);
