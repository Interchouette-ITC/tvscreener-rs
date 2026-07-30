// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Asset-class screeners and shared [`Screener`] builder state.
//!
//! [`Screener::build_payload`] assembles the POST body; [`Screener::get`] sends it and
//! decodes rows. Filters with the same `left` key merge: new values append and the
//! operator promotes to [`FilterOperator::InRange`] when values are not a subset.

#[macro_use]
mod typed;

pub mod bond;
pub mod coin;
pub mod crypto;
pub mod forex;
pub mod futures;
pub mod stock;

use crate::error::{Result, TvscreenerError};
use crate::field::FieldDef;
use crate::filter::{ExtraFilter, FieldCondition, Filter, FilterOperator};
use crate::util::{
    get_columns_to_request, get_url, is_status_code_ok, request_headers, DEFAULT_TIMEOUT_SECS,
    MIN_STREAM_INTERVAL_SECS,
};
use crate::ScreenerRow;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::time::Duration;

/// Default result window start
pub const DEFAULT_MIN_RANGE: u32 = 0;

/// Default result window end
pub const DEFAULT_MAX_RANGE: u32 = 150;

fn build_http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(timeout)
        .default_headers(request_headers())
        .build()
        .expect("reqwest Client builder must succeed")
}

const HTTP_ERROR_BODY_MAX: usize = 4096;

/// Truncates an HTTP error body for [`TvscreenerError::HttpStatus`] display.
pub(crate) fn truncate_http_body(body: String) -> String {
    if body.len() <= HTTP_ERROR_BODY_MAX {
        body
    } else {
        let mut truncated = body;
        truncated.truncate(HTTP_ERROR_BODY_MAX);
        truncated.push('…');
        truncated
    }
}

/// Request builder for one scanner POST.
///
/// Holds columns, filters, range, sort, markets/symbols, and a reused
/// [`reqwest::Client`]. [`Self::build_payload`] builds the JSON body;
/// [`Self::get`] POSTs to `scanner.tradingview.com/{subtype}/scan` and returns
/// [`ScreenerRow`]s. Prefer typed wrappers (`StockScreener`, `CryptoScreener`, …)
/// which set subtype and defaults; use this type when you need the raw builder.
///
/// Override the timeout with [`Self::set_timeout`] before [`Self::get`].
///
/// # Payload example
///
/// ```
/// use tvscreener::core::Screener;
/// use tvscreener::field::{index_symbol, FieldDef, Market};
/// use tvscreener::filter::{FieldCondition, FilterOperator};
/// use serde_json::json;
///
/// let mut screener = Screener::new("global");
/// screener
///.select([FieldDef::new("Name", "name")])
///.set_markets([Market::america().value])
///.set_index([index_symbol::SP500])
///.where_condition(FieldCondition::new(
/// "close",
/// FilterOperator::Above,
/// json!(50),
/// )).unwrap()
///.set_range(0, 10);
/// let payload = screener.build_payload().unwrap();
/// assert_eq!(payload["markets"], json!(["america"]));
/// assert_eq!(payload["symbols"]["symbolset"], json!(["SYML:SP;SPX"]));
/// ```
#[derive(Debug, Clone)]
pub struct Screener {
    /// Path segment in `https://scanner.tradingview.com/{subtype}/scan`.
    pub subtype: String,
    url: Option<String>,
    filters: Vec<Filter>,
    options: HashMap<String, Value>,
    misc: HashMap<String, Value>,
    symbols: Option<Value>,
    selected_fields: Vec<FieldDef>,
    select_all_fields: bool,
    all_fields_fn: Option<fn() -> Vec<FieldDef>>,
    range_from: u32,
    range_to: u32,
    sort_by: Option<(String, bool)>,
    print_request: bool,
    /// When true, filter `left` keys must appear in the allowed field set.
    validate_filter_fields: bool,
    markets: Option<Vec<String>>,
    timeout: Duration,
    client: reqwest::Client,
}

impl Screener {
    /// Creates a screener targeting `subtype` (e.g. `"crypto"`, `"global"` for stocks).
    pub fn new(subtype: impl Into<String>) -> Self {
        let timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS);
        let mut screener = Self {
            subtype: subtype.into(),
            url: None,
            filters: Vec::new(),
            options: HashMap::new(),
            misc: HashMap::new(),
            symbols: None,
            selected_fields: Vec::new(),
            select_all_fields: false,
            all_fields_fn: None,
            range_from: DEFAULT_MIN_RANGE,
            range_to: DEFAULT_MAX_RANGE,
            sort_by: None,
            print_request: false,
            validate_filter_fields: false,
            markets: None,
            timeout,
            client: build_http_client(timeout),
        };
        screener.add_option("lang", json!("en"));
        screener.set_range(DEFAULT_MIN_RANGE, DEFAULT_MAX_RANGE);
        screener
    }

    /// Overrides the HTTP request timeout (default: [`DEFAULT_TIMEOUT_SECS`]).
    ///
    /// Rebuilds the shared [`reqwest::Client`] so the new timeout applies to later
    /// [`Self::get`] / stream calls.
    pub fn set_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self.client = build_http_client(timeout);
        self
    }

    /// Returns the configured HTTP timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Overrides the POST URL (defaults to [`crate::util::get_url`] for [`Self::subtype`]).
    pub fn set_url(&mut self, url: impl Into<String>) -> &mut Self {
        self.url = Some(url.into());
        self
    }

    /// Registers the callback used by [`Self::select_all`]
    pub fn set_all_fields_fn(&mut self, f: fn() -> Vec<FieldDef>) -> &mut Self {
        self.all_fields_fn = Some(f);
        self
    }

    /// Restricts requested columns, replacing any prior selection
    pub fn select(&mut self, fields: impl IntoIterator<Item = FieldDef>) -> &mut Self {
        self.selected_fields = fields.into_iter().collect();
        self.select_all_fields = false;
        self
    }

    /// Requests the field set registered by [`Self::set_all_fields_fn`]
    ///
    /// # Errors
    ///
    /// Returns [`TvscreenerError::InvalidRequest`] when `set_all_fields_fn` was not called.
    pub fn select_all(&mut self) -> Result<&mut Self> {
        let f = self.all_fields_fn.ok_or_else(|| {
            TvscreenerError::InvalidRequest(
                "select_all requires set_all_fields_fn on this screener".into(),
            )
        })?;
        self.selected_fields = f();
        self.select_all_fields = true;
        Ok(self)
    }

    fn find_filter_index(&self, left: &str) -> Option<usize> {
        self.filters.iter().position(|f| f.left == left)
    }

    fn merge_filters(current: &mut Filter, new_filter: &Filter) {
        if !values_are_subset(&new_filter.values, &current.values) {
            current.operation = FilterOperator::InRange;
            for value in &new_filter.values {
                if !current.values.iter().any(|v| v == value) {
                    current.values.push(value.clone());
                }
            }
        }
    }

    fn add_new_filter(&mut self, mut filter: Filter) {
        if filter.values.len() == 1 && filter.operation == FilterOperator::InRange {
            filter.operation = FilterOperator::Equal;
        }
        self.filters.push(filter);
    }

    /// Returns whether `left` is allowed when [`Self::validate_filter_fields`] is enabled.
    ///
    /// Always allows [`ExtraFilter`] keys and stock structural keys `type` / `subtype`.
    /// Otherwise requires a match against [`Self::set_all_fields_fn`] defaults when set,
    /// else the currently selected columns' technical names.
    #[must_use]
    pub fn filter_left_allowed(&self, left: &str) -> bool {
        if left == "type"
            || left == "subtype"
            || left == ExtraFilter::CurrentTradingDay.field_name()
            || left == ExtraFilter::Search.field_name()
            || left == ExtraFilter::Primary.field_name()
        {
            return true;
        }
        let allowed: Vec<String> = self.all_fields_fn.map_or_else(
            || {
                self.selected_fields
                    .iter()
                    .map(|field| field.field_name.clone())
                    .collect()
            },
            |f| f().into_iter().map(|field| field.field_name).collect(),
        );
        allowed.iter().any(|name| name == left)
    }

    fn ensure_filter_left_allowed(&self, left: &str) -> Result<()> {
        if !self.validate_filter_fields || self.filter_left_allowed(left) {
            return Ok(());
        }
        Err(TvscreenerError::InvalidFieldName(left.to_string()))
    }

    /// Enables or disables filter-field validation (default: off).
    pub const fn set_validate_filter_fields(&mut self, enable: bool) -> &mut Self {
        self.validate_filter_fields = enable;
        self
    }

    /// Returns whether filter `left` keys are validated against the allowed field set.
    #[must_use]
    pub const fn validate_filter_fields(&self) -> bool {
        self.validate_filter_fields
    }

    /// Appends or merges a filter clause.
    ///
    /// # Errors
    ///
    /// Returns [`TvscreenerError::InvalidFieldName`] when validation is enabled and `left`
    /// is not in the allowed set.
    pub fn add_filter(
        &mut self,
        left: impl Into<String>,
        operation: FilterOperator,
        values: impl IntoIterator<Item = Value>,
    ) -> Result<&mut Self> {
        let left = left.into();
        self.ensure_filter_left_allowed(&left)?;
        let new_filter = Filter::new(left.clone(), operation, values);
        if let Some(index) = self.find_filter_index(&left) {
            Self::merge_filters(&mut self.filters[index], &new_filter);
        } else {
            self.add_new_filter(new_filter);
        }
        Ok(self)
    }

    /// Appends or merges a filter on an [`ExtraFilter`] left key.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::add_filter`] errors (extra keys are always allowed).
    pub fn add_extra_filter(
        &mut self,
        extra: ExtraFilter,
        operation: FilterOperator,
        values: impl IntoIterator<Item = Value>,
    ) -> Result<&mut Self> {
        let filter = Filter::on_extra(extra, operation, values);
        self.add_filter(filter.left, filter.operation, filter.values)
    }

    /// Appends or merges a filter from a [`FieldCondition`].
    ///
    /// # Errors
    ///
    /// Propagates [`Self::add_filter`] errors.
    pub fn where_condition(&mut self, condition: FieldCondition) -> Result<&mut Self> {
        let filter = condition.into_filter();
        self.add_filter(filter.left, filter.operation, filter.values)
    }

    /// Adds a name/description search filter.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::add_extra_filter`] errors.
    pub fn search(&mut self, value: impl Into<String>) -> Result<&mut Self> {
        self.add_extra_filter(
            ExtraFilter::Search,
            FilterOperator::Match,
            [json!(value.into())],
        )
    }

    /// Removes the first filter whose `left` matches
    pub fn remove_filter(&mut self, left: &str) -> &mut Self {
        if let Some(index) = self.find_filter_index(left) {
            self.filters.remove(index);
        }
        self
    }

    /// Sets the `[from, to)` result window
    pub const fn set_range(&mut self, from_range: u32, to_range: u32) -> &mut Self {
        self.range_from = from_range;
        self.range_to = to_range;
        self
    }

    /// Sets sort column and direction by technical field name
    pub fn sort_by(&mut self, field_name: impl Into<String>, ascending: bool) -> &mut Self {
        self.sort_by = Some((field_name.into(), ascending));
        self
    }

    /// Sets sort column from a [`FieldDef`] (uses [`FieldDef::field_name`]).
    pub fn sort_by_field(&mut self, field: &FieldDef, ascending: bool) -> &mut Self {
        self.sort_by(field.field_name.as_str(), ascending)
    }

    /// Enables debug logging of scan URL and JSON payload (via `tracing` at DEBUG).
    ///
    /// Also enabled for the process when `TVSCREENER_DEBUG=1` (payload dump) after
    /// [`crate::logging::init_logging`], or when `RUST_LOG` includes `tvscreener=debug`.
    pub const fn set_debug(&mut self, enable: bool) -> &mut Self {
        self.print_request = enable;
        self
    }

    /// Alias for [`Self::set_debug`].
    pub const fn set_print_request(&mut self, enable: bool) -> &mut Self {
        self.set_debug(enable)
    }

    /// Adds one scan `options` entry
    pub fn add_option(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        self.options.insert(key.into(), value);
        self
    }

    /// Adds one top-level payload key outside `filter`/`columns`
    pub fn add_misc(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        self.misc.insert(key.into(), value);
        self
    }

    /// Restricts results to index symbol sets.
    ///
    /// Catalog values like `"SP;SPX"` are sent as `"SYML:SP;SPX"`. Values already prefixed with `SYML:` are
    /// left unchanged.
    pub fn set_index(
        &mut self,
        symbolsets: impl IntoIterator<Item = impl Into<String>>,
    ) -> &mut Self {
        let symbolset: Vec<String> = symbolsets
            .into_iter()
            .map(|s| crate::field::to_symbolset_wire(&s.into()))
            .collect();
        if symbolset.is_empty() {
            return self;
        }
        match &mut self.symbols {
            Some(Value::Object(map)) => {
                map.insert("symbolset".into(), json!(symbolset));
            }
            _ => {
                self.symbols = Some(json!({ "symbolset": symbolset }));
            }
        }
        self
    }

    /// Sets the `symbols` block on the payload
    pub fn set_symbols(&mut self, symbols: Value) -> &mut Self {
        self.symbols = Some(symbols);
        self
    }

    /// Sets market filters included as top-level `"markets"` (stock screener).
    pub fn set_markets(
        &mut self,
        markets: impl IntoIterator<Item = impl Into<String>>,
    ) -> &mut Self {
        self.markets = Some(markets.into_iter().map(Into::into).collect());
        self
    }

    /// Returns configured filters (read-only view for tests and debugging).
    #[must_use]
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    /// Returns whether `select_all` was enabled.
    #[must_use]
    pub const fn is_select_all(&self) -> bool {
        self.select_all_fields
    }

    /// Returns the configured result range `(from, to)`.
    #[must_use]
    pub const fn range(&self) -> (u32, u32) {
        (self.range_from, self.range_to)
    }

    /// Returns the selected field definitions.
    #[must_use]
    pub fn selected_fields(&self) -> &[FieldDef] {
        &self.selected_fields
    }

    fn request_url(&self) -> String {
        self.url.clone().unwrap_or_else(|| get_url(&self.subtype))
    }

    fn resolved_fields(&self) -> Result<Vec<FieldDef>> {
        if self.selected_fields.is_empty() {
            return Err(TvscreenerError::InvalidRequest(
                "no fields selected; call select() or select_all()".into(),
            ));
        }
        Ok(self.selected_fields.clone())
    }

    fn build_sort(&self) -> Option<Value> {
        self.sort_by.as_ref().map(|(field, ascending)| {
            json!({
                "sortBy": field,
                "sortOrder": if *ascending { "asc" } else { "desc" },
            })
        })
    }

    /// Builds the JSON POST body
    ///
    /// # Errors
    ///
    /// Returns [`TvscreenerError::InvalidRequest`] when no fields were selected.
    pub fn build_payload(&self) -> Result<Value> {
        let fields = self.resolved_fields()?;
        let columns = get_columns_to_request(&fields);
        let requested_columns: Vec<&str> = columns.iter().map(|(tech, _)| tech.as_str()).collect();

        let mut payload = json!({
            "filter": self.filters.iter().map(Filter::to_json).collect::<Vec<_>>(),
            "options": self.options,
            "symbols": self.symbols.clone().unwrap_or_else(|| {
                json!({ "query": { "types": [] }, "tickers": [] })
            }),
            "sort": self.build_sort(),
            "range": [self.range_from, self.range_to],
            "columns": requested_columns,
        });

        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in &self.misc {
                obj.insert(key.clone(), value.clone());
            }
            if let Some(markets) = &self.markets {
                obj.insert("markets".into(), json!(markets));
            }
        }

        Ok(payload)
    }

    /// Parses a scanner JSON body into [`ScreenerRow`] values keyed by column labels.
    ///
    /// # Errors
    ///
    /// Returns [`TvscreenerError::Json`] when `data`, row symbols, or row value arrays
    /// are missing or malformed.
    pub fn parse_rows(body: &Value, columns: &[(String, String)]) -> Result<Vec<ScreenerRow>> {
        let data = body
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| TvscreenerError::Json("response missing data array".into()))?;

        let mut rows = Vec::with_capacity(data.len());
        for entry in data {
            let symbol = entry
                .get("s")
                .and_then(Value::as_str)
                .ok_or_else(|| TvscreenerError::Json("row missing symbol".into()))?
                .to_string();
            let values = entry
                .get("d")
                .and_then(Value::as_array)
                .ok_or_else(|| TvscreenerError::Json("row missing data array".into()))?;

            let mut map = Map::new();
            for ((_, label), value) in columns.iter().zip(values.iter()) {
                map.insert(label.clone(), value.clone());
            }
            rows.push(ScreenerRow { symbol, data: map });
        }
        Ok(rows)
    }

    /// Runs the scan and returns decoded rows
    ///
    /// # Errors
    ///
    /// Propagates [`TvscreenerError::InvalidRequest`], HTTP/network failures,
    /// non-success status codes, JSON parse errors, and row decode failures.
    pub async fn get(&self) -> Result<Vec<ScreenerRow>> {
        let fields = self.resolved_fields()?;
        let columns = get_columns_to_request(&fields);
        let payload = self.build_payload()?;
        let url = self.request_url();

        let debug = self.print_request || crate::logging::env_debug_enabled();
        if debug {
            tracing::debug!(%url, "screener POST");
            tracing::debug!(payload = %serde_json::to_string_pretty(&payload)?, "screener payload");
        } else {
            tracing::trace!(%url, "screener POST");
        }

        let client = &self.client;
        let response = client.post(&url).json(&payload).send().await?;
        let status = response.status().as_u16();
        let body_text = response.text().await?;

        if !is_status_code_ok(status) {
            tracing::warn!(%status, body_len = body_text.len(), "screener HTTP error");
            return Err(TvscreenerError::HttpStatus {
                status,
                body: truncate_http_body(body_text),
            });
        }

        tracing::debug!(%status, body_len = body_text.len(), "screener HTTP ok");
        let body: Value = serde_json::from_str(&body_text)?;
        Self::parse_rows(&body, &columns)
    }

    /// Polls [`Self::get`] on an interval, invoking `on_update` for each successful batch.
    pub async fn stream_with_callback<F>(
        &self,
        interval_secs: f64,
        max_iterations: Option<usize>,
        mut on_update: F,
    ) where
        F: FnMut(&[ScreenerRow]),
    {
        let interval_secs = interval_secs.max(MIN_STREAM_INTERVAL_SECS);
        let mut iteration = 0usize;
        loop {
            if max_iterations.is_some_and(|max| iteration >= max) {
                break;
            }

            match self.get().await {
                Ok(rows) => {
                    tracing::debug!(iteration, rows = rows.len(), "stream batch");
                    on_update(&rows);
                }
                Err(err) => tracing::warn!(iteration, error = %err, "stream get failed"),
            }

            iteration += 1;
            if max_iterations.is_some_and(|max| iteration >= max) {
                break;
            }
            tokio::time::sleep(Duration::from_secs_f64(interval_secs)).await;
        }
    }

    /// Polls [`Self::get`] and returns each successful batch (errors are logged and skipped).
    pub async fn stream(
        &self,
        interval_secs: f64,
        max_iterations: Option<usize>,
    ) -> Vec<Vec<ScreenerRow>> {
        let mut collected = Vec::new();
        self.stream_with_callback(interval_secs, max_iterations, |rows| {
            collected.push(rows.to_vec());
        })
        .await;
        collected
    }
}

/// Configures a typed screener for `asset` and returns the result of `f`.
///
/// # Errors
///
/// Propagates errors from `f` or typed screener construction defaults.
pub fn with_asset_screener<R, F>(asset: crate::field::Asset, f: F) -> Result<R>
where
    F: FnOnce(&mut Screener) -> Result<R>,
{
    use crate::field::Asset;
    match asset {
        Asset::Stock => {
            let mut s = stock::StockScreener::new();
            f(s.inner_mut())
        }
        Asset::Crypto => {
            let mut s = crypto::CryptoScreener::new();
            f(s.inner_mut())
        }
        Asset::Forex => {
            let mut s = forex::ForexScreener::new();
            f(s.inner_mut())
        }
        Asset::Bond => {
            let mut s = bond::BondScreener::new();
            f(s.inner_mut())
        }
        Asset::Futures => {
            let mut s = futures::FuturesScreener::new();
            f(s.inner_mut())
        }
        Asset::Coin => {
            let mut s = coin::CoinScreener::new();
            f(s.inner_mut())
        }
    }
}

/// Configures a typed screener for `asset`, then runs [`Screener::get`].
///
/// # Errors
///
/// Propagates configure or HTTP/scan failures.
pub async fn get_for_asset<F>(asset: crate::field::Asset, configure: F) -> Result<Vec<ScreenerRow>>
where
    F: FnOnce(&mut Screener) -> Result<()>,
{
    let screener = with_asset_screener(asset, |inner| {
        configure(inner)?;
        Ok(inner.clone())
    })?;
    screener.get().await
}

fn values_are_subset(candidate: &[Value], current: &[Value]) -> bool {
    candidate
        .iter()
        .all(|value| current.iter().any(|v| v == value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{crypto_rsi_14, default_crypto_fields};
    use crate::filter::ExtraFilter;
    use serde_json::json;

    #[test]
    fn builder_stores_range_and_filter() {
        let mut s = Screener::new("crypto");
        s.set_range(10, 20)
            .where_condition(FieldCondition::new(
                "volume",
                FilterOperator::Above,
                json!(1_000_000),
            ))
            .unwrap();
        assert_eq!(s.range(), (10, 20));
        assert_eq!(s.filters().len(), 1);
    }

    #[test]
    fn merge_same_left_extends_values_and_promotes_in_range() {
        let mut s = Screener::new("crypto");
        s.add_filter("type", FilterOperator::Equal, [json!("stock")])
            .unwrap();
        s.add_filter("type", FilterOperator::Equal, [json!("fund")])
            .unwrap();
        assert_eq!(s.filters().len(), 1);
        assert_eq!(s.filters()[0].operation, FilterOperator::InRange);
        assert_eq!(s.filters()[0].values.len(), 2);
    }

    #[test]
    fn single_in_range_becomes_equal() {
        let mut s = Screener::new("crypto");
        s.add_filter("type", FilterOperator::InRange, [json!("stock")])
            .unwrap();
        assert_eq!(s.filters()[0].operation, FilterOperator::Equal);
    }

    #[test]
    fn search_adds_match_filter() {
        let mut s = Screener::new("crypto");
        s.search("btc").unwrap();
        assert_eq!(s.filters()[0].left, ExtraFilter::Search.field_name());
        assert_eq!(s.filters()[0].operation, FilterOperator::Match);
    }

    #[test]
    fn build_payload_crypto_defaults_shape() {
        let mut s = Screener::new("crypto");
        s.select(default_crypto_fields())
            .add_filter("name", FilterOperator::Match, [json!("btc")])
            .unwrap()
            .sort_by("24h_vol|5", false)
            .set_range(0, 5);
        let payload = s.build_payload().expect("payload");
        assert!(payload.get("filter").and_then(Value::as_array).is_some());
        assert!(payload.get("columns").and_then(Value::as_array).is_some());
        assert_eq!(payload["range"], json!([0, 5]));
        assert_eq!(payload["options"]["lang"], json!("en"));
    }

    #[test]
    fn build_payload_includes_markets_when_set() {
        let mut s = Screener::new("global");
        s.select([FieldDef::new("Name", "name")])
            .set_markets(["america"]);
        let payload = s.build_payload().expect("payload");
        assert_eq!(payload["markets"], json!(["america"]));
    }

    #[test]
    fn select_replaces_previous_fields() {
        let mut s = Screener::new("crypto");
        s.select([FieldDef::new("Name", "name")]);
        s.select([FieldDef::new("Price", "close")]);
        assert_eq!(s.selected_fields().len(), 1);
        assert_eq!(s.selected_fields()[0].field_name, "close");
    }

    #[test]
    fn timeout_default_and_override() {
        let mut s = Screener::new("crypto");
        assert_eq!(s.timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        s.set_timeout(Duration::from_secs(5));
        assert_eq!(s.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn sort_by_field_uses_technical_name() {
        let mut s = Screener::new("crypto");
        let rsi = crypto_rsi_14();
        s.select([rsi.clone()]).sort_by_field(&rsi, true);
        let payload = s.build_payload().expect("payload");
        assert_eq!(payload["sort"]["sortBy"], json!("RSI"));
        assert_eq!(payload["sort"]["sortOrder"], json!("asc"));
    }

    #[test]
    fn validate_filter_fields_rejects_unknown_left() {
        let mut s = Screener::new("crypto");
        s.set_all_fields_fn(default_crypto_fields)
            .select(default_crypto_fields())
            .set_validate_filter_fields(true);
        let err = s
            .add_filter("not_a_real_field_xyz", FilterOperator::Equal, [json!(1)])
            .expect_err("unknown left");
        assert!(matches!(err, TvscreenerError::InvalidFieldName(_)));
        s.add_filter("name", FilterOperator::Match, [json!("btc")])
            .expect("name is in crypto defaults");
        s.search("eth").expect("search extra filter always allowed");
        s.add_filter("type", FilterOperator::Equal, [json!("stock")])
            .expect("structural type always allowed");
    }

    #[test]
    fn set_index_symbolset_wire_shape() {
        let mut s = Screener::new("global");
        s.select([FieldDef::new("Name", "name")])
            .set_index([crate::field::index_symbol::SP500]);
        let payload = s.build_payload().expect("payload");
        assert_eq!(payload["symbols"]["symbolset"], json!(["SYML:SP;SPX"]));
    }

    #[test]
    fn set_index_does_not_double_prefix_syml() {
        let mut s = Screener::new("global");
        s.select([FieldDef::new("Name", "name")])
            .set_index(["SYML:SP;SPX", crate::field::index_symbol::NASDAQ_100]);
        let payload = s.build_payload().expect("payload");
        assert_eq!(
            payload["symbols"]["symbolset"],
            json!(["SYML:SP;SPX", "SYML:NASDAQ;NDX"])
        );
    }

    #[test]
    fn truncate_http_body_caps_at_4kib() {
        let short = truncate_http_body("ok".into());
        assert_eq!(short, "ok");
        let long = "x".repeat(HTTP_ERROR_BODY_MAX + 10);
        let truncated = truncate_http_body(long);
        assert_eq!(truncated.len(), HTTP_ERROR_BODY_MAX + '…'.len_utf8());
        assert!(truncated.ends_with('…'));
    }
}
