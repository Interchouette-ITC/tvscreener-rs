// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Shared scan configuration, payload build, and codegen for the TUI.

use serde_json::Value;

use crate::core::{with_asset_screener, Screener};
use crate::field::{list_presets, Asset};
use crate::resolve::apply_stock_index_markets;
use crate::Result;

use super::model::ScanConfig;

/// Preset names for an asset (`stock_*`, `crypto_*`, …) plus a defaults entry.
#[must_use]
pub fn preset_options(asset: Asset) -> Vec<String> {
    let prefix = format!("{}_", asset.as_str());
    let mut names: Vec<String> = list_presets()
        .into_iter()
        .filter(|name| name.starts_with(&prefix))
        .collect();
    names.sort();
    names.insert(0, "(defaults)".into());
    names
}

/// Index of `preset` in [`preset_options`] (`0` when unset or unknown).
#[must_use]
pub fn preset_index(asset: Asset, preset: Option<&str>) -> usize {
    let options = preset_options(asset);
    preset.map_or(0, |name| {
        options.iter().position(|p| p == name).unwrap_or(0)
    })
}

/// Sets `config.preset` from a preset picker index.
pub fn apply_preset_index(config: &mut ScanConfig, index: usize) {
    let options = preset_options(config.asset);
    let picked = options.get(index).map(String::as_str);
    config.preset = match picked {
        None | Some("(defaults)") => None,
        Some(name) => Some(name.to_string()),
    };
}

/// Configures a screener from TUI scan parameters.
///
/// # Errors
///
/// Propagates field resolution, filter, search, and stock market errors.
pub fn configure_screener(inner: &mut Screener, config: &ScanConfig, debug: bool) -> Result<()> {
    let fields = config.resolve_fields()?;
    if debug || crate::logging::env_debug_enabled() {
        inner.set_debug(true);
    }
    inner
        .select(fields)
        .set_range(config.from, config.from.saturating_add(config.limit));
    if let Some(q) = config.search.as_deref().filter(|s| !s.is_empty()) {
        inner.search(q)?;
    }
    for condition in &config.filters {
        inner.where_condition(condition.clone())?;
    }
    if matches!(config.asset, Asset::Stock) {
        apply_stock_index_markets(inner, config.index.as_deref(), config.markets.as_deref())?;
    } else if config.markets.is_some() || config.index.is_some() {
        return Err(crate::TvscreenerError::InvalidRequest(
            "--markets / --index only apply to stock".into(),
        ));
    }
    Ok(())
}

/// Builds the scanner POST JSON for the current config (no network).
///
/// # Errors
///
/// Propagates [`configure_screener`] and [`Screener::build_payload`] errors.
pub fn build_payload(config: &ScanConfig, debug: bool) -> Result<Value> {
    with_asset_screener(config.asset, |inner| {
        configure_screener(inner, config, debug)?;
        inner.build_payload()
    })
}

/// Pretty JSON for the payload pane.
///
/// # Errors
///
/// Propagates [`build_payload`] or JSON serialization errors.
pub fn payload_pretty(config: &ScanConfig, debug: bool) -> Result<String> {
    let value = build_payload(config, debug)?;
    serde_json::to_string_pretty(&value).map_err(Into::into)
}

/// Rust + CLI snippets for the codegen pane.
#[must_use]
pub fn render_codegen(config: &ScanConfig) -> String {
    let mut out = String::new();
    out.push_str("// Rust\n");
    out.push_str(&render_rust_snippet(config));
    out.push_str("\n\n# CLI\n");
    out.push_str(&render_cli_command(config));
    out
}

const fn typed_screener_name(asset: Asset) -> &'static str {
    match asset {
        Asset::Stock => "StockScreener",
        Asset::Crypto => "CryptoScreener",
        Asset::Forex => "ForexScreener",
        Asset::Bond => "BondScreener",
        Asset::Futures => "FuturesScreener",
        Asset::Coin => "CoinScreener",
    }
}

fn render_rust_snippet(config: &ScanConfig) -> String {
    let screener = typed_screener_name(config.asset);
    let mut lines = vec![
        "use serde_json::json;".to_string(),
        format!("use tvscreener::core::{screener};"),
        "use tvscreener::filter::{FieldCondition, FilterOperator};".to_string(),
    ];
    if config.preset.is_some() {
        lines.push("use tvscreener::field::get_preset;".to_string());
    }
    if matches!(config.asset, Asset::Stock) && (config.index.is_some() || config.markets.is_some())
    {
        lines.push("use tvscreener::resolve::apply_stock_index_markets;".to_string());
    }
    lines.push(String::new());
    lines.push(format!("let mut screener = {screener}::new();"));
    if let Some(preset) = config.preset.as_deref() {
        lines.push(format!("let fields = get_preset({preset:?})?;"));
        lines.push("screener.select(fields);".to_string());
    }
    lines.push(format!(
        "screener.set_range({}, {});",
        config.from,
        config.from.saturating_add(config.limit)
    ));
    if let Some(q) = config.search.as_deref().filter(|s| !s.is_empty()) {
        lines.push(format!("screener.search({q:?})?;"));
    }
    for f in &config.filters {
        lines.push(format!(
            "screener.where_condition(FieldCondition::new({:?}, FilterOperator::{:?}, json!({})))?;",
            f.left,
            f.operation,
            filter_value_literal(&f.value)
        ));
    }
    if matches!(config.asset, Asset::Stock) {
        if let Some(index) = config.index.as_deref() {
            lines.push(format!(
                "apply_stock_index_markets(screener.inner_mut(), Some({index:?}), None)?;"
            ));
        }
        if let Some(markets) = config.markets.as_deref() {
            let index_arg = config
                .index
                .as_deref()
                .map_or_else(|| "None".into(), |i| format!("Some({i:?})"));
            lines.push(format!(
                "apply_stock_index_markets(screener.inner_mut(), {index_arg}, Some({markets:?}))?;"
            ));
        }
    }
    lines.push("let rows = screener.get().await?;".to_string());
    lines.join("\n")
}

fn filter_value_literal(value: &Value) -> String {
    match value {
        Value::String(s) => format!("{s:?}"),
        other => other.to_string(),
    }
}

fn render_cli_command(config: &ScanConfig) -> String {
    let mut parts = vec![
        "tvscreener-tui".to_string(),
        config.asset.as_str().to_string(),
    ];
    if let Some(preset) = &config.preset {
        parts.push(format!("--preset {preset}"));
    }
    if config.from != 0 {
        parts.push(format!("--from {}", config.from));
    }
    if config.limit != 25 {
        parts.push(format!("--limit {}", config.limit));
    }
    if let Some(q) = config.search.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("--search {q:?}"));
    }
    if let Some(markets) = &config.markets {
        parts.push(format!("--markets {markets}"));
    }
    if let Some(index) = &config.index {
        parts.push(format!("--index {index}"));
    }
    // Field filters are not exposed on the CLI yet; note equivalent scan command.
    let mut scan = parts.clone();
    scan[0] = "tvscreener".into();
    scan.insert(1, "scan".into());
    if config.filters.is_empty() {
        parts.join(" ")
    } else {
        format!(
            "{}\n# filters: use Rust API or TUI builder (CLI filter flags not yet available)\n{}",
            parts.join(" "),
            scan.join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{default_fields, get_preset};
    use crate::filter::{FieldCondition, FilterOperator};
    use serde_json::json;

    fn sample_config() -> ScanConfig {
        ScanConfig {
            asset: Asset::Crypto,
            preset: Some("crypto_price".into()),
            from: 0,
            limit: 10,
            search: Some("BTC".into()),
            markets: None,
            index: None,
            filters: vec![FieldCondition::new(
                "close",
                FilterOperator::Above,
                json!(50_000),
            )],
        }
    }

    #[test]
    fn preset_options_start_with_defaults() {
        let opts = preset_options(Asset::Crypto);
        assert_eq!(opts[0], "(defaults)");
        assert!(opts.iter().any(|n| n == "crypto_price"));
    }

    #[test]
    fn build_payload_includes_filter() {
        let config = sample_config();
        let payload = build_payload(&config, false).expect("payload");
        let filters = payload["filter"].as_array().expect("filter array");
        assert!(filters.iter().any(|f| f["left"] == "close"));
    }

    #[test]
    fn codegen_mentions_typed_screener_and_cli() {
        let text = render_codegen(&sample_config());
        assert!(text.contains("CryptoScreener"));
        assert!(text.contains("tvscreener-tui"));
        assert!(text.contains("where_condition"));
    }

    #[test]
    fn apply_preset_index_clears_on_defaults() {
        let mut config = sample_config();
        apply_preset_index(&mut config, 0);
        assert!(config.preset.is_none());
    }

    #[test]
    fn resolve_fields_uses_preset() {
        let fields = sample_config().resolve_fields().expect("fields");
        assert!(!fields.is_empty());
        let preset_only = get_preset("crypto_price").expect("preset");
        assert_eq!(fields.len(), preset_only.len());
        assert_ne!(fields, default_fields(Asset::Crypto));
    }
}
