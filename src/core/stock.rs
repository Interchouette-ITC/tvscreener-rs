// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! [`StockScreener`] (`TradingView` subtype `global`).
//!
//! Default stock fields, markets, and symbol-type filters.

use super::Screener;
use crate::error::{Result, TvscreenerError};
use crate::field::{
    default_stock_fields, stock_market_capitalization, symbol_type, type_filter, Market,
    SymbolType, TypeFilter,
};
use crate::filter::FilterOperator;
use serde_json::json;
use std::collections::HashSet;

const SYMBOL_TYPE_TO_TYPE: &[(&str, &str)] = &[
    ("COMMON_STOCK", "STOCK"),
    ("DEPOSITORY_RECEIPT", "DEPOSITORY_RECEIPT"),
    ("ETF", "FUND"),
    ("MUTUAL_FUND", "FUND"),
    ("REIT", "FUND"),
    ("PREFERRED_STOCK", "STOCK"),
    ("ETN", "STRUCTURED"),
    ("STRUCTURED", "STRUCTURED"),
    ("UIT", "FUND"),
];

/// Stock screener with asset-class defaults
#[derive(Debug, Clone)]
pub struct StockScreener {
    inner: Screener,
}

impl StockScreener {
    /// Creates a screener with default stock fields, America market, and market-cap sort.
    pub fn new() -> Self {
        let mut inner = Screener::new("global");
        inner.set_url(crate::util::get_url("global"));
        inner.set_all_fields_fn(default_stock_fields);
        inner.select(default_stock_fields());
        inner.set_markets([Market::america().value]);
        inner.sort_by(stock_market_capitalization().field_name, false);
        Self { inner }
    }

    /// Sets market filters
    pub fn set_markets(&mut self, markets: impl IntoIterator<Item = Market>) -> &mut Self {
        let selected: Vec<Market> = markets.into_iter().collect();
        let wire_values: Vec<String> = if selected.iter().any(|m| m.const_name == "ALL") {
            crate::field::all_markets()
                .iter()
                .map(|m| m.value.clone())
                .collect()
        } else {
            selected.into_iter().map(|m| m.value).collect()
        };
        self.inner.set_markets(wire_values);
        self
    }

    /// Adds stock type and subtype filters
    ///
    /// # Errors
    ///
    /// Returns [`TvscreenerError::InvalidRequest`] when a symbol type is unknown or
    /// has no type mapping.
    pub fn set_symbol_types(
        &mut self,
        symbol_types: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<&mut Self> {
        let mut requested = Vec::new();
        for name in symbol_types {
            let const_name = name.as_ref();
            let st = symbol_type(const_name).ok_or_else(|| {
                TvscreenerError::InvalidRequest(format!("unknown symbol type: {const_name}"))
            })?;
            requested.push(st);
        }

        let type_values = collect_type_filters(&requested)?;
        emit_type_filters(&mut self.inner, &type_values);
        emit_subtype_filters(&mut self.inner, requested);

        Ok(self)
    }
}

fn collect_type_filters(requested: &[SymbolType]) -> Result<Vec<TypeFilter>> {
    let mut types_to_add = HashSet::new();
    for st in requested {
        let mapped = SYMBOL_TYPE_TO_TYPE
            .iter()
            .find(|(sym, _)| *sym == st.const_name)
            .map(|(_, ty)| *ty)
            .ok_or_else(|| {
                TvscreenerError::InvalidRequest(format!(
                    "symbol type `{}` has no type mapping",
                    st.const_name
                ))
            })?;
        types_to_add.insert(mapped.to_string());
    }
    Ok(types_to_add
        .into_iter()
        .filter_map(|name| type_filter(&name))
        .collect())
}

fn emit_type_filters(inner: &mut Screener, type_values: &[TypeFilter]) {
    if type_values.len() > 1 {
        inner
            .add_filter(
                "type",
                FilterOperator::InRange,
                type_values.iter().map(|t| json!(t.value)),
            )
            .expect("type is always an allowed filter left");
    } else if let Some(one) = type_values.first() {
        inner
            .add_filter("type", FilterOperator::Equal, [json!(one.value)])
            .expect("type is always an allowed filter left");
    }
}

fn emit_subtype_filters(inner: &mut Screener, mut symbol_types_to_apply: Vec<SymbolType>) {
    if symbol_types_to_apply
        .iter()
        .any(|st| st.const_name == "COMMON_STOCK")
        && !symbol_types_to_apply
            .iter()
            .any(|st| st.const_name == "DEPOSITORY_RECEIPT")
    {
        if let Some(dr) = symbol_type("DEPOSITORY_RECEIPT") {
            symbol_types_to_apply.push(dr);
        }
    }

    for st in symbol_types_to_apply {
        inner
            .add_filter(
                "subtype",
                FilterOperator::InRange,
                st.values.iter().map(|value| json!(value)),
            )
            .expect("subtype is always an allowed filter left");
    }
}

impl_typed_screener_common!(StockScreener);
