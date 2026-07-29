// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Token parsing and wire-value resolution for screener parameters.

use crate::core::Screener;
use crate::error::{Result, TvscreenerError};
use crate::field::{
    all_index_symbols, all_markets, index_symbol_value, market, resolve_sector, Asset,
};
use crate::filter::FilterOperator;

fn parse_csv_tokens(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// Parses a comma-separated field name list into a `Vec<String>`.
pub(crate) fn parse_field_list(fields: Option<&str>) -> Vec<String> {
    parse_csv_tokens(fields)
}

/// Parses an optional `"min,max"` range string into `(min, max)` floats.
///
/// Empty sides become `None`. Invalid numbers are ignored for that side.
#[must_use]
pub fn parse_f64_range(raw: Option<&str>) -> (Option<f64>, Option<f64>) {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    let mut parts = raw.splitn(2, ',');
    let min = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());
    let max = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());
    (min, max)
}

/// Maps operator strings to [`FilterOperator`] variants.
#[must_use]
pub fn parse_filter_op(op: &str) -> Option<FilterOperator> {
    match op.trim() {
        ">=" | "egreater" | "above_or_equal" => Some(FilterOperator::AboveOrEqual),
        ">" | "greater" | "above" => Some(FilterOperator::Above),
        "<=" | "eless" | "below_or_equal" => Some(FilterOperator::BelowOrEqual),
        "<" | "less" | "below" => Some(FilterOperator::Below),
        "==" | "=" | "equal" => Some(FilterOperator::Equal),
        "!=" | "nequal" | "not_equal" => Some(FilterOperator::NotEqual),
        "match" => Some(FilterOperator::Match),
        "in_range" => Some(FilterOperator::InRange),
        "not_in_range" => Some(FilterOperator::NotInRange),
        "crosses" => Some(FilterOperator::Crosses),
        "crosses_up" | "crosses_above" => Some(FilterOperator::CrossesUp),
        "crosses_down" | "crosses_below" => Some(FilterOperator::CrossesDown),
        _ => FilterOperator::from_wire(op.trim()),
    }
}

/// Parses an asset type string into [`Asset`].
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown asset types.
pub fn parse_asset(asset_type: &str) -> Result<Asset> {
    Asset::parse(asset_type.trim().to_ascii_lowercase().as_str()).ok_or_else(|| {
        TvscreenerError::InvalidRequest(format!(
            "unknown asset_type `{asset_type}` (use stock|crypto|forex|bond|futures|coin)"
        ))
    })
}

/// Resolves index CSV tokens (const name or wire) to symbolset wire values.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown tokens.
pub fn resolve_index_wires(indices: Option<&str>) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for token in parse_csv_tokens(indices) {
        let catalog = token.strip_prefix("SYML:").unwrap_or(token.as_str());
        let upper = catalog.to_ascii_uppercase().replace(' ', "_");
        if let Some(wire) = index_symbol_value(&upper) {
            out.push(wire.to_string());
            continue;
        }
        if let Some(idx) = all_index_symbols()
            .iter()
            .find(|idx| idx.value.eq_ignore_ascii_case(catalog))
        {
            out.push(idx.value.clone());
            continue;
        }
        return Err(TvscreenerError::InvalidRequest(format!(
            "unknown index `{token}` (use list_index_symbols)"
        )));
    }
    Ok(out)
}

/// Resolves market CSV tokens (const name or wire) to market wire values.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown tokens.
pub fn resolve_market_wires(markets: Option<&str>) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for token in parse_csv_tokens(markets) {
        let upper = token.to_ascii_uppercase().replace(' ', "_");
        if let Some(m) = market(&upper) {
            out.push(m.value);
            continue;
        }
        if let Some(m) = all_markets()
            .iter()
            .find(|m| m.value.eq_ignore_ascii_case(&token))
        {
            out.push(m.value.clone());
            continue;
        }
        return Err(TvscreenerError::InvalidRequest(format!(
            "unknown market `{token}` (use list_markets)"
        )));
    }
    Ok(out)
}

/// Resolves sector CSV tokens (const name or wire) to sector wire values.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown tokens.
pub fn resolve_sector_wires(sectors: Option<&str>) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for token in parse_csv_tokens(sectors) {
        let Some(wire) = resolve_sector(&token) else {
            return Err(TvscreenerError::InvalidRequest(format!(
                "unknown sector `{token}` (use list_sectors)"
            )));
        };
        out.push(wire.to_string());
    }
    Ok(out)
}

/// Applies resolved index symbolsets and market filters to a stock screener.
///
/// # Errors
///
/// Returns errors from [`resolve_index_wires`] or [`resolve_market_wires`].
pub(crate) fn apply_stock_index_markets(
    screener: &mut Screener,
    indices: Option<&str>,
    markets: Option<&str>,
) -> Result<()> {
    let wires = resolve_index_wires(indices)?;
    if !wires.is_empty() {
        screener.set_index(wires);
    }
    let market_wires = resolve_market_wires(markets)?;
    if !market_wires.is_empty() {
        screener.set_markets(market_wires);
    }
    Ok(())
}
