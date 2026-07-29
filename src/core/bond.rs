// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! [`BondScreener`] (`TradingView` subtype `bond`).
//!
//! Default bond fields (or `bond_basic` preset fallback) and volume sort.

use super::Screener;
use crate::field::{bond_volume, default_bond_fields};

/// Bond screener with asset-class defaults
#[derive(Debug, Clone)]
pub struct BondScreener {
    inner: Screener,
}

impl BondScreener {
    /// Creates a screener with default bond fields and descending volume sort.
    pub fn new() -> Self {
        let mut inner = Screener::new("bond");
        inner.set_url(crate::util::get_url("bond"));
        inner.set_all_fields_fn(default_bond_fields);
        inner.select(default_bond_fields());
        inner.sort_by(bond_volume().field_name, false);
        Self { inner }
    }
}

impl_typed_screener_common!(BondScreener);
