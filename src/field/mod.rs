// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Curated field definitions, presets, and metadata loaded from `data/fields.json`.
//!
//! Each column is `Field(label, field_name, format, interval, historical)`
//! in `tvscreener.field`. This crate uses [`FieldDef`] plus a generated catalog.

use crate::error::{Result, TvscreenerError};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Asset class key in the generated catalog (`crypto`, `stock`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asset {
    /// Cryptocurrency screener fields.
    Crypto,
    /// Global stock screener fields.
    Stock,
    /// Forex pair fields.
    Forex,
    /// Bond fields.
    Bond,
    /// Futures contract fields.
    Futures,
    /// CEX/DEX coin fields.
    Coin,
}

impl Asset {
    /// Wire/catalog segment for this asset class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Crypto => "crypto",
            Self::Stock => "stock",
            Self::Forex => "forex",
            Self::Bond => "bond",
            Self::Futures => "futures",
            Self::Coin => "coin",
        }
    }

    /// Parses an asset segment from catalog or preset references.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "crypto" => Some(Self::Crypto),
            "stock" => Some(Self::Stock),
            "forex" => Some(Self::Forex),
            "bond" => Some(Self::Bond),
            "futures" => Some(Self::Futures),
            "coin" => Some(Self::Coin),
            _ => None,
        }
    }
}

/// `TradingView` market filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Market {
    /// Catalog constant name (e.g. `"AMERICA"`).
    pub const_name: String,
    /// Wire value sent in scan payloads (e.g. `"america"`).
    pub value: String,
}

/// Named catalog constant with a single wire/display string (sector, country, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedValue {
    /// Catalog constant name (e.g. `"TECHNOLOGY_SERVICES"`).
    pub const_name: String,
    /// Wire / display value (e.g. `"Technology Services"`).
    pub value: String,
}

/// Recommendation rating band (numeric recommendation scores).
#[derive(Debug, Clone, Copy)]
pub struct RatingBand {
    /// Catalog constant name (e.g. `"STRONG_BUY"`).
    pub const_name: &'static str,
    /// Inclusive lower bound (`f64::NAN` for [`rating::UNKNOWN`]).
    pub min: f64,
    /// Inclusive upper bound (`f64::NAN` for [`rating::UNKNOWN`]).
    pub max: f64,
    /// Human label (e.g. `"Strong Buy"`).
    pub label: &'static str,
}

impl RatingBand {
    /// Returns whether `value` falls in `[min, max]` (false when bounds are NaN).
    #[must_use]
    pub fn contains(self, value: f64) -> bool {
        if self.min.is_nan() || self.max.is_nan() || value.is_nan() {
            return false;
        }
        self.min <= value && value <= self.max
    }

    /// Returns `[min, max]` when both bounds are finite.
    #[must_use]
    pub const fn range(self) -> Option<[f64; 2]> {
        if self.min.is_nan() || self.max.is_nan() {
            None
        } else {
            Some([self.min, self.max])
        }
    }
}

impl Market {
    /// Convenience constant for the US market.
    ///
    /// # Panics
    ///
    /// Panics if `AMERICA` is missing from the embedded catalog.
    #[must_use]
    pub fn america() -> Self {
        market("AMERICA").expect("AMERICA market must exist in fields.json")
    }

    /// Convenience constant for all markets.
    ///
    /// # Panics
    ///
    /// Panics if `ALL` is missing from the embedded catalog.
    #[must_use]
    pub fn all() -> Self {
        market("ALL").expect("ALL market must exist in fields.json")
    }
}

/// Stock `type` filter value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeFilter {
    /// Catalog constant name (e.g. `"STOCK"`).
    pub const_name: String,
    /// Wire value (e.g. `"stock"`).
    pub value: String,
}

/// Stock `subtype` filter value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolType {
    /// Catalog constant name (e.g. `"COMMON_STOCK"`).
    pub const_name: String,
    /// Wire values merged into `in_range` filters.
    pub values: Vec<String>,
}

/// One requestable scanner column
///
/// Field order:
/// `(label, field_name, format, interval, historical)`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldDef {
    /// Human-readable column title
    pub label: String,
    /// `TradingView` technical name used in payloads and filters
    pub field_name: String,
    /// Optional display/format hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// When true, the field supports time-interval variants
    pub interval: bool,
    /// When true, historical lookback columns may be requested
    pub historical: bool,
}

impl FieldDef {
    /// Creates a field with default `format`, `interval`, and `historical` unset/false.
    pub fn new(label: impl Into<String>, field_name: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            field_name: field_name.into(),
            format: None,
            interval: false,
            historical: false,
        }
    }

    /// Returns whether this field uses recommendation format (`format == "recommendation"`).
    #[must_use]
    pub fn has_recommendation(&self) -> bool {
        self.format.as_deref() == Some("recommendation")
    }
}

#[derive(serde::Deserialize)]
struct RawCatalogEntry {
    label: String,
    field_name: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    interval: bool,
    #[serde(default)]
    historical: bool,
}

#[derive(serde::Deserialize)]
struct RawFieldsJson {
    defaults: HashMap<String, Vec<String>>,
    catalog: HashMap<String, HashMap<String, RawCatalogEntry>>,
    presets: HashMap<String, Vec<String>>,
    markets: Vec<RawMarket>,
    types: Vec<RawType>,
    symbol_types: Vec<RawSymbolType>,
    #[serde(default)]
    index_symbols: Vec<RawIndexSymbol>,
    #[serde(default)]
    sectors: Vec<RawNamedValue>,
    #[serde(default)]
    countries: Vec<RawNamedValue>,
    #[serde(default)]
    industries: Vec<RawNamedValue>,
    #[serde(default)]
    exchanges: Vec<RawNamedValue>,
}

#[derive(serde::Deserialize)]
struct RawMarket {
    #[serde(rename = "const")]
    const_name: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct RawType {
    #[serde(rename = "const")]
    const_name: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct RawSymbolType {
    #[serde(rename = "const")]
    const_name: String,
    value: Vec<String>,
}

#[derive(serde::Deserialize)]
struct RawIndexSymbol {
    #[serde(rename = "const")]
    const_name: String,
    value: String,
    #[serde(default)]
    label: String,
}

#[derive(serde::Deserialize)]
struct RawNamedValue {
    #[serde(rename = "const")]
    const_name: String,
    value: String,
}

/// One index filter entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexSymbolDef {
    /// Catalog constant name (e.g. `"SP500"`).
    pub const_name: String,
    /// Catalog symbol (e.g. `"SP;SPX"`); payload uses [`to_symbolset_wire`].
    pub value: String,
    /// Human label (e.g. `"S&P 500"`).
    pub label: String,
}

struct FieldsCatalog {
    defaults: HashMap<String, Vec<String>>,
    catalog: HashMap<String, HashMap<String, FieldDef>>,
    presets: HashMap<String, Vec<String>>,
    markets: Vec<Market>,
    types: Vec<TypeFilter>,
    symbol_types: Vec<SymbolType>,
    index_symbols: Vec<IndexSymbolDef>,
    sectors: Vec<NamedValue>,
    countries: Vec<NamedValue>,
    industries: Vec<NamedValue>,
    exchanges: Vec<NamedValue>,
}

fn map_named(raw: Vec<RawNamedValue>) -> Vec<NamedValue> {
    raw.into_iter()
        .map(|n| NamedValue {
            const_name: n.const_name,
            value: n.value,
        })
        .collect()
}

fn find_named<'a>(items: &'a [NamedValue], const_name: &str) -> Option<&'a NamedValue> {
    items.iter().find(|n| n.const_name == const_name)
}

fn named_value_str<'a>(items: &'a [NamedValue], const_name: &str) -> Option<&'a str> {
    find_named(items, const_name).map(|n| n.value.as_str())
}

fn resolve_named_wire<'a>(items: &'a [NamedValue], token: &str) -> Option<&'a str> {
    let upper = token.to_ascii_uppercase().replace(' ', "_");
    if let Some(v) = named_value_str(items, &upper) {
        return Some(v);
    }
    items
        .iter()
        .find(|n| n.value.eq_ignore_ascii_case(token))
        .map(|n| n.value.as_str())
}

impl FieldsCatalog {
    fn load() -> Self {
        let raw: RawFieldsJson = serde_json::from_str(include_str!("../../data/fields.json"))
            .expect("fields.json must deserialize");

        let catalog = raw
            .catalog
            .into_iter()
            .map(|(asset, entries)| {
                let mapped = entries
                    .into_iter()
                    .map(|(name, entry)| {
                        (
                            name,
                            FieldDef {
                                label: entry.label,
                                field_name: entry.field_name,
                                format: entry.format,
                                interval: entry.interval,
                                historical: entry.historical,
                            },
                        )
                    })
                    .collect();
                (asset, mapped)
            })
            .collect();

        Self {
            defaults: raw.defaults,
            presets: raw.presets,
            catalog,
            markets: raw
                .markets
                .into_iter()
                .map(|m| Market {
                    const_name: m.const_name,
                    value: m.value,
                })
                .collect(),
            types: raw
                .types
                .into_iter()
                .map(|t| TypeFilter {
                    const_name: t.const_name,
                    value: t.value,
                })
                .collect(),
            symbol_types: raw
                .symbol_types
                .into_iter()
                .map(|st| SymbolType {
                    const_name: st.const_name,
                    values: st.value,
                })
                .collect(),
            index_symbols: raw
                .index_symbols
                .into_iter()
                .map(|idx| IndexSymbolDef {
                    const_name: idx.const_name,
                    value: idx.value,
                    label: idx.label,
                })
                .collect(),
            sectors: map_named(raw.sectors),
            countries: map_named(raw.countries),
            industries: map_named(raw.industries),
            exchanges: map_named(raw.exchanges),
        }
    }

    fn field(&self, asset: Asset, const_name: &str) -> Option<FieldDef> {
        self.catalog.get(asset.as_str())?.get(const_name).cloned()
    }

    fn defaults_for(&self, asset: Asset) -> Vec<FieldDef> {
        self.defaults
            .get(asset.as_str())
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.field(asset, name))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn resolve_preset_ref(&self, reference: &str) -> Option<FieldDef> {
        let (asset_str, const_name) = reference.split_once('.')?;
        let asset = Asset::parse(asset_str)?;
        self.field(asset, const_name)
    }

    fn preset(&self, name: &str) -> Result<Vec<FieldDef>> {
        let refs = self
            .presets
            .get(name)
            .ok_or_else(|| TvscreenerError::InvalidRequest(format!("unknown preset: {name}")))?;
        let fields: Vec<FieldDef> = refs
            .iter()
            .filter_map(|reference| self.resolve_preset_ref(reference))
            .collect();
        if fields.is_empty() {
            return Err(TvscreenerError::InvalidRequest(format!(
                "preset `{name}` resolved to no fields"
            )));
        }
        Ok(fields)
    }

    fn search(&self, asset: Asset, query: &str) -> Vec<FieldDef> {
        let query = query.to_ascii_lowercase();
        self.catalog
            .get(asset.as_str())
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(name, field)| {
                        name.to_ascii_lowercase().contains(&query)
                            || field.label.to_ascii_lowercase().contains(&query)
                            || field.field_name.to_ascii_lowercase().contains(&query)
                    })
                    .map(|(_, field)| field.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

static CATALOG: OnceLock<FieldsCatalog> = OnceLock::new();

fn catalog() -> &'static FieldsCatalog {
    CATALOG.get_or_init(FieldsCatalog::load)
}

/// Looks up one catalog field by asset and constant name.
#[must_use]
pub fn field(asset: Asset, const_name: &str) -> Option<FieldDef> {
    catalog().field(asset, const_name)
}

/// Resolves a field by constant name, technical name, or label (case-insensitive).
///
/// Prefer exact constant-name match (e.g. `"PRICE"`), then technical `field_name`,
/// then label. Returns `(const_name, FieldDef)` when found.
#[must_use]
pub fn resolve_field(asset: Asset, name: &str) -> Option<(String, FieldDef)> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase().replace(' ', "_");
    if let Some(def) = catalog().field(asset, &upper) {
        return Some((upper, def));
    }
    let entries = catalog().catalog.get(asset.as_str())?;
    let lower = trimmed.to_ascii_lowercase();
    entries.iter().find_map(|(const_name, def)| {
        if def.field_name.eq_ignore_ascii_case(trimmed)
            || def.label.to_ascii_lowercase() == lower
            || const_name.eq_ignore_ascii_case(&upper)
        {
            Some((const_name.clone(), def.clone()))
        } else {
            None
        }
    })
}

/// Returns a catalog field or an error when it is missing.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when the constant name is absent for the asset.
pub fn require_field(asset: Asset, const_name: &str) -> Result<FieldDef> {
    field(asset, const_name).ok_or_else(|| {
        TvscreenerError::InvalidRequest(format!(
            "unknown field `{const_name}` for asset `{}`",
            asset.as_str()
        ))
    })
}

/// Returns preset field lists by name
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when the preset is unknown or resolves to no fields.
pub fn get_preset(name: &str) -> Result<Vec<FieldDef>> {
    catalog().preset(name)
}

/// Lists known preset names sorted alphabetically
#[must_use]
pub fn list_presets() -> Vec<String> {
    let mut names: Vec<String> = catalog().presets.keys().cloned().collect();
    names.sort();
    names
}

/// Searches catalog fields for an asset by label, constant name, or technical name.
#[must_use]
pub fn search_fields(asset: Asset, query: &str) -> Vec<FieldDef> {
    catalog().search(asset, query)
}

/// Returns every field definition for an asset (full catalog, not only defaults).
#[must_use]
pub fn list_fields(asset: Asset) -> Vec<FieldDef> {
    catalog()
        .catalog
        .get(asset.as_str())
        .map(|entries| entries.values().cloned().collect())
        .unwrap_or_default()
}

/// Returns how many fields are registered for an asset in the embedded catalog.
#[must_use]
pub fn catalog_len(asset: Asset) -> usize {
    catalog()
        .catalog
        .get(asset.as_str())
        .map_or(0, HashMap::len)
}

/// Returns all index symbol definitions from the catalog.
#[must_use]
pub fn all_index_symbols() -> &'static [IndexSymbolDef] {
    &catalog().index_symbols
}

/// Looks up an index catalog symbol by constant name (e.g. `"SP500"` → `"SP;SPX"`).
#[must_use]
pub fn index_symbol_value(const_name: &str) -> Option<&'static str> {
    catalog()
        .index_symbols
        .iter()
        .find(|idx| idx.const_name == const_name)
        .map(|idx| idx.value.as_str())
}

/// Formats a catalog index symbol for the scan `symbols.symbolset` array.
///
/// Index symbolset wire value (`SYML:` + catalog symbol).
#[must_use]
pub fn to_symbolset_wire(symbol: &str) -> String {
    if symbol.starts_with("SYML:") {
        symbol.to_string()
    } else {
        format!("SYML:{symbol}")
    }
}

/// Returns all configured markets from the catalog.
#[must_use]
pub fn all_markets() -> &'static [Market] {
    &catalog().markets
}

/// Finds a market by catalog constant name.
#[must_use]
pub fn market(const_name: &str) -> Option<Market> {
    catalog()
        .markets
        .iter()
        .find(|m| m.const_name == const_name)
        .cloned()
}

/// Returns all sector catalog entries.
#[must_use]
pub fn all_sectors() -> &'static [NamedValue] {
    &catalog().sectors
}

/// Sector wire value by constant name (e.g. `"TECHNOLOGY_SERVICES"`).
#[must_use]
pub fn sector_value(const_name: &str) -> Option<&'static str> {
    named_value_str(&catalog().sectors, const_name)
}

/// Resolves a sector token (const name or wire value) to the catalog wire string.
#[must_use]
pub fn resolve_sector(token: &str) -> Option<&'static str> {
    resolve_named_wire(&catalog().sectors, token)
}

/// Returns all country catalog entries.
#[must_use]
pub fn all_countries() -> &'static [NamedValue] {
    &catalog().countries
}

/// Country wire value by constant name.
#[must_use]
pub fn country_value(const_name: &str) -> Option<&'static str> {
    named_value_str(&catalog().countries, const_name)
}

/// Resolves a country token (const name or wire value).
#[must_use]
pub fn resolve_country(token: &str) -> Option<&'static str> {
    resolve_named_wire(&catalog().countries, token)
}

/// Returns all industry catalog entries.
#[must_use]
pub fn all_industries() -> &'static [NamedValue] {
    &catalog().industries
}

/// Industry wire value by constant name.
#[must_use]
pub fn industry_value(const_name: &str) -> Option<&'static str> {
    named_value_str(&catalog().industries, const_name)
}

/// Resolves an industry token (const name or wire value).
#[must_use]
pub fn resolve_industry(token: &str) -> Option<&'static str> {
    resolve_named_wire(&catalog().industries, token)
}

/// Returns all exchange catalog entries.
#[must_use]
pub fn all_exchanges() -> &'static [NamedValue] {
    &catalog().exchanges
}

/// Exchange wire value by constant name.
#[must_use]
pub fn exchange_value(const_name: &str) -> Option<&'static str> {
    named_value_str(&catalog().exchanges, const_name)
}

/// Resolves an exchange token (const name or wire value).
#[must_use]
pub fn resolve_exchange(token: &str) -> Option<&'static str> {
    resolve_named_wire(&catalog().exchanges, token)
}

/// Returns recommendation rating bands (same order as [`rating::ALL`]).
#[must_use]
pub const fn all_ratings() -> &'static [RatingBand] {
    rating::ALL
}

/// Looks up a rating band by constant name.
#[must_use]
pub fn rating(const_name: &str) -> Option<RatingBand> {
    rating::ALL
        .iter()
        .copied()
        .find(|r| r.const_name == const_name)
}

/// Finds the rating band containing `value`, or [`rating::UNKNOWN`].
#[must_use]
pub fn rating_find(value: f64) -> RatingBand {
    for band in rating::ALL {
        if band.const_name == "UNKNOWN" {
            continue;
        }
        if band.contains(value) {
            return *band;
        }
    }
    rating::UNKNOWN
}

/// Finds a stock `type` filter by catalog constant name.
#[must_use]
pub fn type_filter(const_name: &str) -> Option<TypeFilter> {
    catalog()
        .types
        .iter()
        .find(|t| t.const_name == const_name)
        .cloned()
}

/// Finds a stock symbol subtype by catalog constant name.
#[must_use]
pub fn symbol_type(const_name: &str) -> Option<SymbolType> {
    catalog()
        .symbol_types
        .iter()
        .find(|st| st.const_name == const_name)
        .cloned()
}

/// Default crypto columns (`DEFAULT_CRYPTO_FIELDS`).
#[must_use]
pub fn default_crypto_fields() -> Vec<FieldDef> {
    catalog().defaults_for(Asset::Crypto)
}

/// Default stock columns (`DEFAULT_STOCK_FIELDS`).
#[must_use]
pub fn default_stock_fields() -> Vec<FieldDef> {
    catalog().defaults_for(Asset::Stock)
}

/// Default forex columns (`DEFAULT_FOREX_FIELDS`).
#[must_use]
pub fn default_forex_fields() -> Vec<FieldDef> {
    catalog().defaults_for(Asset::Forex)
}

/// Default bond columns; falls back to `bond_basic` preset when the catalog default list is empty.
#[must_use]
pub fn default_bond_fields() -> Vec<FieldDef> {
    let fields = catalog().defaults_for(Asset::Bond);
    if fields.is_empty() {
        get_preset("bond_basic").unwrap_or_default()
    } else {
        fields
    }
}

/// Default futures columns (`DEFAULT_FUTURES_FIELDS`).
#[must_use]
pub fn default_futures_fields() -> Vec<FieldDef> {
    catalog().defaults_for(Asset::Futures)
}

/// Default coin columns (`DEFAULT_COIN_FIELDS`).
#[must_use]
pub fn default_coin_fields() -> Vec<FieldDef> {
    catalog().defaults_for(Asset::Coin)
}

/// Default columns for `asset` (same as the typed screener constructors).
#[must_use]
pub fn default_fields(asset: Asset) -> Vec<FieldDef> {
    match asset {
        Asset::Stock => default_stock_fields(),
        Asset::Crypto => default_crypto_fields(),
        Asset::Forex => default_forex_fields(),
        Asset::Bond => default_bond_fields(),
        Asset::Futures => default_futures_fields(),
        Asset::Coin => default_coin_fields(),
    }
}

/// Crypto sort field: 24h volume in USD.
///
/// # Panics
///
/// Panics if `VOLUME_24H_IN_USD` is missing from the embedded catalog.
#[must_use]
pub fn crypto_volume_24h_in_usd() -> FieldDef {
    require_field(Asset::Crypto, "VOLUME_24H_IN_USD").expect("VOLUME_24H_IN_USD")
}

/// Stock sort field: market capitalization.
///
/// # Panics
///
/// Panics if `MARKET_CAPITALIZATION` is missing from the embedded catalog.
#[must_use]
pub fn stock_market_capitalization() -> FieldDef {
    require_field(Asset::Stock, "MARKET_CAPITALIZATION").expect("MARKET_CAPITALIZATION")
}

/// Forex sort field: name.
///
/// # Panics
///
/// Panics if `NAME` is missing from the embedded catalog.
#[must_use]
pub fn forex_name() -> FieldDef {
    require_field(Asset::Forex, "NAME").expect("NAME")
}

/// Bond sort field: volume.
///
/// # Panics
///
/// Panics if `VOLUME` is missing from the embedded catalog.
#[must_use]
pub fn bond_volume() -> FieldDef {
    require_field(Asset::Bond, "VOLUME").expect("VOLUME")
}

/// Futures sort field: volume.
///
/// # Panics
///
/// Panics if `VOLUME` is missing from the embedded catalog.
#[must_use]
pub fn futures_volume() -> FieldDef {
    require_field(Asset::Futures, "VOLUME").expect("VOLUME")
}

/// Coin sort field: market cap.
///
/// # Panics
///
/// Panics if `MARKET_CAP` is missing from the embedded catalog.
#[must_use]
pub fn coin_market_cap() -> FieldDef {
    require_field(Asset::Coin, "MARKET_CAP").expect("MARKET_CAP")
}

/// Crypto RSI(14) field used in technical presets and filters.
///
/// # Panics
///
/// Panics if `RELATIVE_STRENGTH_INDEX_14` is missing from the embedded catalog.
#[must_use]
pub fn crypto_rsi_14() -> FieldDef {
    require_field(Asset::Crypto, "RELATIVE_STRENGTH_INDEX_14").expect("RELATIVE_STRENGTH_INDEX_14")
}

/// Crypto price (`close`) field.
///
/// # Panics
///
/// Panics if `PRICE` is missing from the embedded catalog.
#[must_use]
pub fn crypto_price() -> FieldDef {
    require_field(Asset::Crypto, "PRICE").expect("PRICE")
}

/// Common index catalog symbols for [`crate::core::Screener::set_index`].
///
/// Generated from `data/fields.json` - run `tvscreener regen-fields` to refresh.
pub mod index_symbol {
    include!("generated/index_symbol.rs");
}

/// Stock sector wire values for `SECTOR` filters.
///
/// Generated from `data/fields.json` - run `tvscreener regen-fields` to refresh.
pub mod sector {
    include!("generated/sector.rs");
}

/// Country wire values for country filters.
///
/// Generated from `data/fields.json` - run `tvscreener regen-fields` to refresh.
pub mod country {
    include!("generated/country.rs");
}

/// Industry wire values for industry filters.
///
/// Generated from `data/fields.json` - run `tvscreener regen-fields` to refresh.
pub mod industry {
    include!("generated/industry.rs");
}

/// Exchange wire values for exchange filters.
///
/// Generated from `data/fields.json` - run `tvscreener regen-fields` to refresh.
pub mod exchange {
    include!("generated/exchange.rs");
}

/// Recommendation rating bands (`STRONG_BUY` … `UNKNOWN`).
///
/// Generated from `data/fields.json` - run `tvscreener regen-fields` to refresh.
pub mod rating {
    use super::RatingBand;
    include!("generated/rating.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_field_const_technical_and_label() {
        let (name, def) = resolve_field(Asset::Crypto, "PRICE").expect("const");
        assert_eq!(name, "PRICE");
        assert_eq!(def.field_name, "close");

        let (name, def) = resolve_field(Asset::Crypto, "close").expect("technical");
        assert_eq!(name, "PRICE");
        assert_eq!(def.field_name, "close");

        let by_label = resolve_field(Asset::Crypto, &def.label);
        assert!(by_label.is_some());
        assert!(resolve_field(Asset::Crypto, "not_a_real_field_xyz").is_none());
    }

    #[test]
    fn require_field_unknown_is_invalid() {
        let err = require_field(Asset::Crypto, "NOPE").expect_err("missing");
        assert!(matches!(err, crate::TvscreenerError::InvalidRequest(_)));
        assert!(err.to_string().contains("crypto"));
    }

    #[test]
    fn default_crypto_fields_non_empty() {
        let fields = default_crypto_fields();
        assert!(fields.len() > 50);
        assert!(fields.iter().any(|f| f.field_name == "name"));
    }

    #[test]
    fn get_preset_crypto_price() {
        let fields = get_preset("crypto_price").expect("preset");
        assert!(fields.iter().any(|f| f.label == "Name"));
        assert_eq!(fields.len(), 7);
    }

    #[test]
    fn list_presets_includes_crypto_price() {
        let names = list_presets();
        assert!(names.contains(&"crypto_price".to_string()));
    }

    #[test]
    fn search_fields_finds_volume() {
        let hits = search_fields(Asset::Crypto, "volume");
        assert!(hits
            .iter()
            .any(|f| f.field_name.contains("vol") || f.label.contains("Volume")));
    }

    #[test]
    fn full_catalog_is_much_larger_than_defaults() {
        assert!(catalog_len(Asset::Crypto) > 3000);
        assert!(catalog_len(Asset::Stock) > 3000);
        assert_eq!(list_fields(Asset::Crypto).len(), catalog_len(Asset::Crypto));
        assert!(default_crypto_fields().len() < catalog_len(Asset::Crypto));
    }

    #[test]
    fn index_symbol_sp500_wire_and_catalog() {
        assert_eq!(index_symbol::SP500, "SP;SPX");
        assert_eq!(index_symbol_value("SP500"), Some("SP;SPX"));
        assert_eq!(to_symbolset_wire("SP;SPX"), "SYML:SP;SPX");
        assert_eq!(to_symbolset_wire("SYML:SP;SPX"), "SYML:SP;SPX");
        assert_eq!(
            all_index_symbols().len(),
            index_symbol::ALL_CONST_NAMES.len()
        );
        assert!(all_index_symbols().len() >= 50);
    }

    #[test]
    fn market_america_value() {
        assert_eq!(Market::america().value, "america");
    }

    #[test]
    fn sector_country_industry_exchange_catalog() {
        assert_eq!(sector::TECHNOLOGY_SERVICES, "Technology Services");
        assert_eq!(
            sector_value("TECHNOLOGY_SERVICES"),
            Some("Technology Services")
        );
        assert_eq!(
            resolve_sector("Technology Services"),
            Some("Technology Services")
        );
        assert_eq!(all_sectors().len(), sector::ALL_CONST_NAMES.len());
        assert_eq!(country::UNITED_STATES, "United States");
        assert_eq!(resolve_country("UNITED_STATES"), Some("United States"));
        assert_eq!(resolve_country("United States"), Some("United States"));
        assert_eq!(all_countries().len(), country::ALL_CONST_NAMES.len());
        assert_eq!(industry::SEMICONDUCTORS, "Semiconductors");
        assert_eq!(resolve_industry("SEMICONDUCTORS"), Some("Semiconductors"));
        assert_eq!(all_industries().len(), industry::ALL_CONST_NAMES.len());
        assert_eq!(exchange::NASDAQ, "NASDAQ");
        assert_eq!(resolve_exchange("NASDAQ"), Some("NASDAQ"));
        assert_eq!(all_exchanges().len(), exchange::ALL_CONST_NAMES.len());
        assert_eq!(all_ratings().len(), rating::ALL_CONST_NAMES.len());
    }

    #[test]
    fn rating_find_bands() {
        assert_eq!(rating_find(0.7).const_name, "STRONG_BUY");
        assert_eq!(rating_find(0.0).const_name, "NEUTRAL");
        assert_eq!(rating_find(-0.9).const_name, "STRONG_SELL");
        assert_eq!(rating_find(f64::NAN).const_name, "UNKNOWN");
        assert_eq!(rating("BUY").map(|r| r.label), Some("Buy"));
        assert_eq!(rating::BUY.range(), Some([0.1, 0.5]));
    }

    #[test]
    fn bond_defaults_or_preset_fallback() {
        let fields = default_bond_fields();
        assert!(!fields.is_empty());
    }
}
