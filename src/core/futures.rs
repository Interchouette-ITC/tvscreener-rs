// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! [`FuturesScreener`] (`TradingView` subtype `futures`).
//!
//! Default futures fields and volume sort.

use super::Screener;
use crate::field::{default_futures_fields, futures_volume};

/// Futures screener with asset-class defaults
#[derive(Debug, Clone)]
pub struct FuturesScreener {
    inner: Screener,
}

impl FuturesScreener {
    /// Creates a screener with default futures fields and descending volume sort.
    pub fn new() -> Self {
        let mut inner = Screener::new("futures");
        inner.set_url(crate::util::get_url("futures"));
        inner.set_all_fields_fn(default_futures_fields);
        inner.select(default_futures_fields());
        inner.sort_by(futures_volume().field_name, false);
        Self { inner }
    }
}

impl_typed_screener_common!(FuturesScreener);
