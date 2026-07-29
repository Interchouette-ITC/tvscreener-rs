// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! [`ForexScreener`] (`TradingView` subtype `forex`).
//!
//! Default forex fields, name sort, and forex symbol query misc.

use super::Screener;
use crate::field::{default_forex_fields, forex_name};
use serde_json::json;

/// Forex screener with asset-class defaults
#[derive(Debug, Clone)]
pub struct ForexScreener {
    inner: Screener,
}

impl ForexScreener {
    /// Creates a screener with default forex fields and ascending name sort.
    pub fn new() -> Self {
        let mut inner = Screener::new("forex");
        inner.set_url(crate::util::get_url("forex"));
        inner.set_all_fields_fn(default_forex_fields);
        inner.select(default_forex_fields());
        inner.sort_by(forex_name().field_name, true);
        inner.add_misc("symbols", json!({ "query": { "types": ["forex"] } }));
        Self { inner }
    }
}

impl_typed_screener_common!(ForexScreener);
