// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Shared CSV token parsing and catalog wire-value resolution (CLI + MCP).

use crate::error::{Result, TvscreenerError};
use crate::field::{
    all_index_symbols, all_markets, index_symbol_value, market, resolve_country, resolve_exchange,
    resolve_industry, resolve_sector,
};

/// Splits an optional comma-separated string into trimmed non-empty tokens.
#[must_use]
pub fn parse_csv_tokens(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
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
            "unknown index `{token}`"
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
            "unknown market `{token}`"
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
    resolve_named_csv(sectors, resolve_sector, "sector")
}

/// Resolves country CSV tokens (const name or wire) to country wire values.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown tokens.
pub fn resolve_country_wires(countries: Option<&str>) -> Result<Vec<String>> {
    resolve_named_csv(countries, resolve_country, "country")
}

/// Resolves industry CSV tokens (const name or wire) to industry wire values.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown tokens.
pub fn resolve_industry_wires(industries: Option<&str>) -> Result<Vec<String>> {
    resolve_named_csv(industries, resolve_industry, "industry")
}

/// Resolves exchange CSV tokens (const name or wire) to exchange wire values.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for unknown tokens.
pub fn resolve_exchange_wires(exchanges: Option<&str>) -> Result<Vec<String>> {
    resolve_named_csv(exchanges, resolve_exchange, "exchange")
}

fn resolve_named_csv(
    raw: Option<&str>,
    resolve: fn(&str) -> Option<&'static str>,
    kind: &str,
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for token in parse_csv_tokens(raw) {
        let Some(wire) = resolve(&token) else {
            return Err(TvscreenerError::InvalidRequest(format!(
                "unknown {kind} `{token}`"
            )));
        };
        out.push(wire.to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_and_index_wires() {
        assert_eq!(
            resolve_market_wires(Some("AMERICA")).unwrap(),
            vec!["america".to_string()]
        );
        assert_eq!(
            resolve_index_wires(Some("SP500")).unwrap(),
            vec!["SP;SPX".to_string()]
        );
        assert!(resolve_market_wires(Some("not-a-market")).is_err());
    }

    #[test]
    fn named_catalog_wires() {
        assert_eq!(
            resolve_sector_wires(Some("TECHNOLOGY_SERVICES")).unwrap(),
            vec!["Technology Services".to_string()]
        );
        assert_eq!(
            resolve_country_wires(Some("UNITED_STATES")).unwrap(),
            vec!["United States".to_string()]
        );
        assert_eq!(
            resolve_industry_wires(Some("SEMICONDUCTORS")).unwrap(),
            vec!["Semiconductors".to_string()]
        );
        assert_eq!(
            resolve_exchange_wires(Some("NASDAQ")).unwrap(),
            vec!["NASDAQ".to_string()]
        );
    }
}
