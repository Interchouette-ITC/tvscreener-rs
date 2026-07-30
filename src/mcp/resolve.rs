// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Token parsing and wire-value resolution for screener parameters.

use crate::error::{Result, TvscreenerError};
use crate::field::Asset;
use crate::filter::FilterOperator;
use crate::resolve::parse_csv_tokens;

pub use crate::resolve::{
    apply_stock_index_markets, resolve_country_wires, resolve_exchange_wires, resolve_index_wires,
    resolve_industry_wires, resolve_market_wires, resolve_sector_wires,
};

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

#[cfg(test)]
mod tests {
    use super::parse_f64_range;

    #[test]
    fn parse_f64_range_sides() {
        assert_eq!(parse_f64_range(None), (None, None));
        assert_eq!(parse_f64_range(Some("")), (None, None));
        assert_eq!(parse_f64_range(Some("10,20")), (Some(10.0), Some(20.0)));
        assert_eq!(parse_f64_range(Some("10,")), (Some(10.0), None));
        assert_eq!(parse_f64_range(Some(",20")), (None, Some(20.0)));
        assert_eq!(parse_f64_range(Some("x,20")), (None, Some(20.0)));
        assert_eq!(parse_f64_range(Some("10,y")), (Some(10.0), None));
    }
}
