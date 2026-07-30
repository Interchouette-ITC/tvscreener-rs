// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Screener query helpers for MCP tools.

use crate::core::crypto::CryptoScreener;
use crate::core::forex::ForexScreener;
use crate::core::stock::StockScreener;
use crate::core::Screener;
use crate::error::{Result, TvscreenerError};
use crate::field::{resolve_field, Asset, FieldDef};
use crate::filter::{FieldCondition, FilterOperator};
use crate::ScreenerRow;
use serde_json::{json, Value};

use super::format::format_rows_markdown;
use super::resolve::{apply_stock_index_markets, parse_asset, parse_field_list, parse_filter_op};

fn require_resolved(asset: Asset, name: &str) -> Result<FieldDef> {
    resolve_field(asset, name)
        .map(|(_, def)| def)
        .ok_or_else(|| {
            TvscreenerError::InvalidRequest(format!(
                "unknown field `{name}` for asset `{}`",
                asset.as_str()
            ))
        })
}

fn parse_filters_arg(filters: Option<&str>) -> Result<Vec<Value>> {
    let Some(raw) = filters.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };
    let parsed: Value = serde_json::from_str(raw)
        .map_err(|err| TvscreenerError::InvalidRequest(format!("filters JSON: {err}")))?;
    match parsed {
        Value::Array(items) => Ok(items),
        Value::Object(_) => Ok(vec![parsed]),
        other => Err(TvscreenerError::InvalidRequest(format!(
            "filters must be a JSON array or object, got {other}"
        ))),
    }
}

fn apply_filters(screener: &mut Screener, asset: Asset, filters: &[Value]) -> Result<()> {
    for entry in filters {
        let field_name = entry
            .get("field")
            .and_then(Value::as_str)
            .ok_or_else(|| TvscreenerError::InvalidRequest("filter missing field".into()))?;
        let op_str = entry.get("op").and_then(Value::as_str).unwrap_or(">=");
        let op = parse_filter_op(op_str).ok_or_else(|| {
            TvscreenerError::InvalidRequest(format!("unknown filter op `{op_str}`"))
        })?;
        let value = entry
            .get("value")
            .cloned()
            .ok_or_else(|| TvscreenerError::InvalidRequest("filter missing value".into()))?;
        let def = require_resolved(asset, field_name)?;
        screener.where_condition(FieldCondition::new(def.field_name, op, value))?;
    }
    Ok(())
}

fn apply_select(screener: &mut Screener, asset: Asset, fields: &[String]) -> Result<()> {
    let mut selected = Vec::new();
    for name in fields {
        selected.push(require_resolved(asset, name)?);
    }
    if !selected.is_empty() {
        screener.select(selected);
    }
    Ok(())
}

fn apply_sort(
    screener: &mut Screener,
    asset: Asset,
    sort_by: Option<&str>,
    ascending: bool,
) -> Result<()> {
    if let Some(name) = sort_by {
        let def = require_resolved(asset, name)?;
        screener.sort_by_field(&def, ascending);
    }
    Ok(())
}

async fn run_on_screener<F>(asset: Asset, configure: F) -> Result<Vec<ScreenerRow>>
where
    F: FnOnce(&mut Screener) -> Result<()>,
{
    crate::core::get_for_asset(asset, configure).await
}

/// Options for [`custom_query`].
#[derive(Debug, Clone, Copy)]
pub struct CustomQueryOpts<'a> {
    /// Asset key (`stock`, `crypto`, …).
    pub asset_type: &'a str,
    /// Comma-separated field const names.
    pub fields: Option<&'a str>,
    /// JSON array of `{field, op, value}` filters.
    pub filters: Option<&'a str>,
    /// Sort field const name.
    pub sort_by: Option<&'a str>,
    /// Sort ascending when true.
    pub ascending: bool,
    /// Row limit (clamped 1..=100).
    pub limit: u32,
}

/// Runs a flexible custom screener query and returns Markdown.
///
/// # Errors
///
/// Propagates invalid args and HTTP/scan failures.
pub async fn custom_query(opts: &CustomQueryOpts<'_>) -> Result<String> {
    let asset = parse_asset(opts.asset_type)?;
    let limit = opts.limit.clamp(1, 100);
    let field_list = parse_field_list(opts.fields);
    let filter_list = parse_filters_arg(opts.filters)?;
    let sort_by = opts.sort_by.map(str::trim).filter(|s| !s.is_empty());

    let rows = run_on_screener(asset, |s| {
        if !field_list.is_empty() {
            apply_select(s, asset, &field_list)?;
        }
        apply_filters(s, asset, &filter_list)?;
        apply_sort(s, asset, sort_by, opts.ascending)?;
        s.set_range(0, limit);
        Ok(())
    })
    .await?;

    Ok(format_rows_markdown(&rows, limit as usize))
}

/// Options for [`search_stocks`].
#[derive(Debug, Clone, Copy)]
pub struct SearchStocksOpts<'a> {
    /// Lower bound for price filter (inclusive).
    pub min_price: Option<f64>,
    /// Upper bound for price filter (inclusive).
    pub max_price: Option<f64>,
    /// Lower bound for market cap in billions (inclusive).
    pub min_market_cap_billions: Option<f64>,
    /// Upper bound for market cap in billions (inclusive).
    pub max_market_cap_billions: Option<f64>,
    /// Comma-separated sector names for `SECTOR match` filters.
    pub sectors: Option<&'a str>,
    /// Sort key: `"price"`, `"volume"`, `"change"`, or `"market_cap"` (default).
    pub sort_by: &'a str,
    /// Row limit (clamped 1..=100).
    pub limit: u32,
}

/// Screens stocks with common price / market-cap / sector filters.
///
/// # Errors
///
/// Propagates scan failures.
pub async fn search_stocks(opts: &SearchStocksOpts<'_>) -> Result<String> {
    let limit = opts.limit.clamp(1, 100);
    let mut ss = StockScreener::new();
    let select = [
        require_resolved(Asset::Stock, "NAME")?,
        require_resolved(Asset::Stock, "PRICE")?,
        require_resolved(Asset::Stock, "CHANGE_PERCENT")?,
        require_resolved(Asset::Stock, "VOLUME")?,
        require_resolved(Asset::Stock, "MARKET_CAPITALIZATION")?,
        require_resolved(Asset::Stock, "PRICE_TO_EARNINGS_RATIO_TTM")?,
        require_resolved(Asset::Stock, "SECTOR")?,
    ];
    ss.select(select);

    let price = require_resolved(Asset::Stock, "PRICE")?;
    let mcap = require_resolved(Asset::Stock, "MARKET_CAPITALIZATION")?;
    let sector = require_resolved(Asset::Stock, "SECTOR")?;

    if let Some(min) = opts.min_price {
        ss.where_condition(FieldCondition::new(
            price.field_name.clone(),
            FilterOperator::AboveOrEqual,
            json!(min),
        ))?;
    }
    if let Some(max) = opts.max_price {
        ss.where_condition(FieldCondition::new(
            price.field_name.clone(),
            FilterOperator::BelowOrEqual,
            json!(max),
        ))?;
    }
    if let Some(min_b) = opts.min_market_cap_billions {
        ss.where_condition(FieldCondition::new(
            mcap.field_name.clone(),
            FilterOperator::AboveOrEqual,
            json!(min_b * 1e9),
        ))?;
    }
    if let Some(max_b) = opts.max_market_cap_billions {
        ss.where_condition(FieldCondition::new(
            mcap.field_name.clone(),
            FilterOperator::BelowOrEqual,
            json!(max_b * 1e9),
        ))?;
    }
    if let Some(sectors) = opts.sectors {
        for wire in crate::mcp::resolve::resolve_sector_wires(Some(sectors))? {
            ss.where_condition(FieldCondition::new(
                sector.field_name.clone(),
                FilterOperator::Match,
                json!(wire),
            ))?;
        }
    }

    let sort_key = match opts.sort_by {
        "price" => "PRICE",
        "volume" => "VOLUME",
        "change" => "CHANGE_PERCENT",
        _ => "MARKET_CAPITALIZATION",
    };
    let sort_field = require_resolved(Asset::Stock, sort_key)?;
    ss.inner_mut().sort_by_field(&sort_field, false);
    ss.set_range(0, limit);
    let rows = ss.get().await?;
    Ok(format_rows_markdown(&rows, limit as usize))
}

/// Screens crypto with volume / market-cap filters.
///
/// # Errors
///
/// Propagates scan failures.
pub async fn search_crypto(
    min_volume_millions: Option<f64>,
    min_market_cap_billions: Option<f64>,
    limit: u32,
) -> Result<String> {
    let limit = limit.clamp(1, 100);
    let mut cs = CryptoScreener::new();
    cs.select([
        require_resolved(Asset::Crypto, "NAME")?,
        require_resolved(Asset::Crypto, "PRICE")?,
        require_resolved(Asset::Crypto, "CHANGE_PERCENT")?,
        require_resolved(Asset::Crypto, "VOLUME_24H_IN_USD")?,
        require_resolved(Asset::Crypto, "MARKET_CAPITALIZATION")?,
    ]);
    if let Some(min_m) = min_volume_millions {
        let vol = require_resolved(Asset::Crypto, "VOLUME_24H_IN_USD")?;
        cs.where_condition(FieldCondition::new(
            vol.field_name,
            FilterOperator::AboveOrEqual,
            json!(min_m * 1e6),
        ))?;
    }
    if let Some(min_b) = min_market_cap_billions {
        let mcap = require_resolved(Asset::Crypto, "MARKET_CAPITALIZATION")?;
        cs.where_condition(FieldCondition::new(
            mcap.field_name,
            FilterOperator::AboveOrEqual,
            json!(min_b * 1e9),
        ))?;
    }
    cs.set_range(0, limit);
    let rows = cs.get().await?;
    Ok(format_rows_markdown(&rows, limit as usize))
}

/// Screens forex pairs.
///
/// # Errors
///
/// Propagates scan failures.
pub async fn search_forex(min_volume_millions: Option<f64>, limit: u32) -> Result<String> {
    let limit = limit.clamp(1, 100);
    let mut fs = ForexScreener::new();
    fs.select([
        require_resolved(Asset::Forex, "NAME")?,
        require_resolved(Asset::Forex, "PRICE")?,
        require_resolved(Asset::Forex, "CHANGE_PERCENT")?,
        require_resolved(Asset::Forex, "VOLUME")?,
    ]);
    if let Some(min_m) = min_volume_millions {
        let vol = require_resolved(Asset::Forex, "VOLUME")?;
        fs.where_condition(FieldCondition::new(
            vol.field_name,
            FilterOperator::AboveOrEqual,
            json!(min_m * 1e6),
        ))?;
    }
    fs.set_range(0, limit);
    let rows = fs.get().await?;
    Ok(format_rows_markdown(&rows, limit as usize))
}

/// Top gainers or losers for stock or crypto.
///
/// # Errors
///
/// Propagates scan failures.
pub async fn get_top_movers(asset_type: &str, direction: &str, limit: u32) -> Result<String> {
    let limit = limit.clamp(1, 50);
    let ascending = direction.eq_ignore_ascii_case("losers");
    let label = if ascending { "Losers" } else { "Gainers" };

    let text = if asset_type.eq_ignore_ascii_case("crypto") {
        let mut cs = CryptoScreener::new();
        cs.select([
            require_resolved(Asset::Crypto, "NAME")?,
            require_resolved(Asset::Crypto, "PRICE")?,
            require_resolved(Asset::Crypto, "CHANGE_PERCENT")?,
            require_resolved(Asset::Crypto, "VOLUME_24H_IN_USD")?,
        ]);
        let change = require_resolved(Asset::Crypto, "CHANGE_PERCENT")?;
        cs.inner_mut().sort_by_field(&change, ascending);
        cs.set_range(0, limit);
        let rows = cs.get().await?;
        format_rows_markdown(&rows, limit as usize)
    } else {
        let mut ss = StockScreener::new();
        ss.select([
            require_resolved(Asset::Stock, "NAME")?,
            require_resolved(Asset::Stock, "PRICE")?,
            require_resolved(Asset::Stock, "CHANGE_PERCENT")?,
            require_resolved(Asset::Stock, "VOLUME")?,
            require_resolved(Asset::Stock, "MARKET_CAPITALIZATION")?,
        ]);
        let change = require_resolved(Asset::Stock, "CHANGE_PERCENT")?;
        ss.inner_mut().sort_by_field(&change, ascending);
        ss.set_range(0, limit);
        let rows = ss.get().await?;
        format_rows_markdown(&rows, limit as usize)
    };

    Ok(format!("Top {label}:\n\n{text}"))
}

/// Options for [`custom_query_payload_preview`] (dry-run scan payload).
#[derive(Debug, Clone, Copy)]
pub struct PayloadPreviewOpts<'a> {
    /// Asset key (`stock`, `crypto`, …).
    pub asset_type: &'a str,
    /// Comma-separated field const names.
    pub fields: Option<&'a str>,
    /// JSON array of `{field, op, value}` filters.
    pub filters: Option<&'a str>,
    /// Sort field const name.
    pub sort_by: Option<&'a str>,
    /// Sort ascending when true.
    pub ascending: bool,
    /// Row limit (clamped 1..=100).
    pub limit: u32,
    /// Stock index CSV (`SP500` or `SP;SPX`).
    pub indices: Option<&'a str>,
    /// Stock markets CSV (`AMERICA` or `america`).
    pub markets: Option<&'a str>,
}

/// Builds a filter-merge / select payload without sending HTTP.
///
/// Optional `indices` / `markets` CSV apply stock `set_index` / `set_markets`.
///
/// # Errors
///
/// Returns errors for unknown fields, indices, markets, or bad filter JSON.
pub fn custom_query_payload_preview(opts: &PayloadPreviewOpts<'_>) -> Result<Value> {
    let asset = parse_asset(opts.asset_type)?;
    let limit = opts.limit.clamp(1, 100);
    let field_list = parse_field_list(opts.fields);
    let filter_list = parse_filters_arg(opts.filters)?;
    let sort_by = opts.sort_by.map(str::trim).filter(|s| !s.is_empty());

    crate::core::with_asset_screener(asset, |screener| {
        if field_list.is_empty() {
            if screener.selected_fields().is_empty() {
                screener.select([FieldDef::new("Name", "name")]);
            }
        } else {
            apply_select(screener, asset, &field_list)?;
        }
        apply_filters(screener, asset, &filter_list)?;
        apply_sort(screener, asset, sort_by, opts.ascending)?;
        if asset == Asset::Stock {
            apply_stock_index_markets(screener, opts.indices, opts.markets)?;
        } else if opts.indices.is_some() || opts.markets.is_some() {
            return Err(TvscreenerError::InvalidRequest(
                "indices/markets only apply to asset_type=stock".into(),
            ));
        }
        screener.set_range(0, limit);
        screener.build_payload()
    })
}

/// Options for [`build_payload`].
#[derive(Debug, Clone, Copy)]
pub struct BuildPayloadOpts<'a> {
    /// Asset key (`stock`, `crypto`, …).
    pub asset_type: &'a str,
    /// Comma-separated field const names.
    pub fields: Option<&'a str>,
    /// JSON array of `{field, op, value}` filters.
    pub filters: Option<&'a str>,
    /// Sort field const name.
    pub sort_by: Option<&'a str>,
    /// Stock index CSV (`SP500` or `SP;SPX`).
    pub indices: Option<&'a str>,
    /// Stock markets CSV (`AMERICA` or `america`).
    pub markets: Option<&'a str>,
}

/// Builds a filter-merge / select payload without sending HTTP.
///
/// # Errors
///
/// Returns errors for unknown fields or bad filter JSON.
pub fn build_payload(opts: &BuildPayloadOpts<'_>) -> Result<String> {
    let payload = custom_query_payload_preview(&PayloadPreviewOpts {
        asset_type: opts.asset_type,
        fields: opts.fields,
        filters: opts.filters,
        sort_by: opts.sort_by,
        ascending: false,
        limit: 25,
        indices: opts.indices,
        markets: opts.markets,
    })?;
    serde_json::to_string_pretty(&payload).map_err(TvscreenerError::from)
}

/// Live stock screen filtered by index symbolset(s) and optional markets.
///
/// # Errors
///
/// Propagates unknown indices/markets and scan failures.
pub async fn search_by_index(
    indices: &str,
    markets: Option<&str>,
    fields: Option<&str>,
    sort_by: Option<&str>,
    limit: u32,
) -> Result<String> {
    let limit = limit.clamp(1, 100);
    let mut ss = StockScreener::new();
    let field_list = parse_field_list(fields);
    if field_list.is_empty() {
        ss.select([
            require_resolved(Asset::Stock, "NAME")?,
            require_resolved(Asset::Stock, "PRICE")?,
            require_resolved(Asset::Stock, "CHANGE_PERCENT")?,
            require_resolved(Asset::Stock, "VOLUME")?,
            require_resolved(Asset::Stock, "MARKET_CAPITALIZATION")?,
            require_resolved(Asset::Stock, "SECTOR")?,
        ]);
    } else {
        apply_select(ss.inner_mut(), Asset::Stock, &field_list)?;
    }
    apply_stock_index_markets(ss.inner_mut(), Some(indices), markets)?;
    apply_sort(
        ss.inner_mut(),
        Asset::Stock,
        sort_by.map(str::trim).filter(|s| !s.is_empty()),
        false,
    )?;
    ss.set_range(0, limit);
    let rows = ss.get().await?;
    Ok(format_rows_markdown(&rows, limit as usize))
}
