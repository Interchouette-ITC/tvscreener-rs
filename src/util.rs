// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! URL helpers, request headers, millify, display formatting, and column-name utilities.

use crate::field::FieldDef;
use crate::ScreenerRow;
use serde_json::Value;

/// Base URL for scan endpoints: `{SCANNER_ORIGIN}/{subtype}/scan`.
pub const SCANNER_ORIGIN: &str = "https://scanner.tradingview.com";

/// `Origin` header value
pub const TRADINGVIEW_ORIGIN: &str = "https://www.tradingview.com";

/// `Referer` header value (`TRADINGVIEW_ORIGIN` + trailing `/`).
pub const TRADINGVIEW_REFERER: &str = "https://www.tradingview.com/";

/// Default HTTP timeout for screener POSTs (seconds).
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Minimum stream refresh interval (seconds).
pub const MIN_STREAM_INTERVAL_SECS: f64 = 1.0;

/// Default `User-Agent` for scanner POSTs.
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

const MILL_NAMES: [&str; 5] = ["", "K", "M", "B", "T"];

/// Build the scan URL for a screener subtype (`"crypto"`, `"global"`, …).
#[must_use]
pub fn get_url(subtype: &str) -> String {
    format!("{SCANNER_ORIGIN}/{subtype}/scan")
}

/// HTTP headers for `TradingView` scanner POSTs.
pub fn request_headers() -> reqwest::header::HeaderMap {
    use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT};

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
    headers.insert(ORIGIN, HeaderValue::from_static(TRADINGVIEW_ORIGIN));
    headers.insert(REFERER, HeaderValue::from_static(TRADINGVIEW_REFERER));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers
}

/// Returns true when `status` is in the HTTP 2xx range
#[must_use]
pub fn is_status_code_ok(status: u16) -> bool {
    (200..300).contains(&status)
}

/// Appends a historical lookback suffix
#[must_use]
pub fn add_historical(field_name: &str, historical: u32) -> String {
    format!("{field_name}[{historical}]")
}

/// Label for a historical column
#[must_use]
pub fn add_historical_to_label(label: &str, historical: u32) -> String {
    if historical == 1 {
        format!("Prev. {label}")
    } else {
        format!("Prev. {historical} {label}")
    }
}

/// Recommendation companion column key
#[must_use]
pub fn add_rec(field_name: &str) -> String {
    format!("Rec.{field_name}")
}

/// Recommendation companion column label
#[must_use]
pub fn add_rec_to_label(label: &str) -> String {
    format!("Reco. {label}")
}

/// Rewrites timed field names for the wire format
#[must_use]
pub fn format_timed_field(field_name: &str) -> String {
    if (field_name.starts_with("change") || field_name.starts_with("relative_volume_intraday"))
        && field_name.contains('.')
    {
        let Some((_, num)) = field_name.split_once('.') else {
            return field_name.to_string();
        };
        if num.chars().all(|c| c.is_ascii_digit()) || num == "1W" || num == "1M" {
            return field_name.replace('.', "|");
        }
    }
    field_name.to_string()
}

/// Assembles `(technical_name, label)` pairs for a scan payload
#[must_use]
pub fn get_columns_to_request(fields: &[FieldDef]) -> Vec<(String, String)> {
    let mut columns: Vec<(String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for field in fields {
        if field.field_name.starts_with("candlestick") {
            continue;
        }
        let tech = format_timed_field(&field.field_name);
        if seen.insert(tech.clone()) {
            columns.push((tech, field.label.clone()));
        }
    }

    if seen.insert("update_mode".into()) {
        columns.push(("update_mode".into(), "Update Mode".into()));
    }

    for field in fields {
        if field.has_recommendation() {
            let tech = add_rec(&field.field_name);
            let label = add_rec_to_label(&field.label);
            if seen.insert(tech.clone()) {
                columns.push((tech, label));
            }
        }
    }

    for field in fields {
        if field.historical {
            let tech = add_historical(&field.field_name, 1);
            let label = add_historical_to_label(&field.label, 1);
            if seen.insert(tech.clone()) {
                columns.push((tech, label));
            }
        }
    }

    columns
}

/// Returns the millify suffix index for a positive magnitude (0 = none, 4 = trillions).
const fn millify_index(n: f64) -> usize {
    if n < 1_000.0 {
        0
    } else if n < 1_000_000.0 {
        1
    } else if n < 1_000_000_000.0 {
        2
    } else if n < 1_000_000_000_000.0 {
        3
    } else {
        4
    }
}

/// Exponent for `10_f64.powi` given a millify suffix index (0–4).
const fn millify_pow_exp(millidx: usize) -> i32 {
    match millidx {
        0 => 0,
        1 => 3,
        2 => 6,
        3 => 9,
        _ => 12,
    }
}

/// Format a magnitude with K / M / B / T suffixes and three decimal places
#[must_use]
pub fn millify(n: f64) -> String {
    let is_negative = n < 0.0;
    let n = n.abs();

    let millidx = millify_index(n);
    let scaled = n / 10_f64.powi(millify_pow_exp(millidx));
    let result = format!("{scaled:.3}{}", MILL_NAMES[millidx]);
    if is_negative {
        format!("-{result}")
    } else {
        result
    }
}

/// Maps a numeric rating to `"S"`, `"N"`, or `"B"`
#[must_use]
pub const fn get_recommendation(rating: f64) -> &'static str {
    if rating < 0.0 {
        "S"
    } else if rating == 0.0 {
        "N"
    } else {
        "B"
    }
}

const BUY_CHAR: &str = "↑ B";
const SELL_CHAR: &str = "↓ S";
const NEUTRAL_CHAR: &str = "- N";

/// Formats a numeric recommendation as an arrow + letter.
#[must_use]
pub fn format_rating(rating: f64) -> String {
    match get_recommendation(rating) {
        "B" => BUY_CHAR.to_string(),
        "S" => SELL_CHAR.to_string(),
        _ => NEUTRAL_CHAR.to_string(),
    }
}

/// Alias for [`format_rating`].
#[must_use]
pub fn format_recommendation(rating: f64) -> String {
    format_rating(rating)
}

/// Formats one JSON cell for display; null becomes `"--"`.
#[must_use]
pub fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "--".into(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), millify),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

/// Formats a [`ScreenerRow`], optionally limiting to `column_labels`.
#[must_use]
pub fn format_row(row: &ScreenerRow, column_labels: Option<&[&str]>) -> String {
    let mut parts: Vec<String> = column_labels.map_or_else(
        || {
            row.data
                .iter()
                .map(|(label, value)| format!("{label}: {}", format_value(value)))
                .collect()
        },
        |labels| {
            labels
                .iter()
                .filter_map(|label| {
                    row.data
                        .get(*label)
                        .map(|value| format!("{label}: {}", format_value(value)))
                })
                .collect()
        },
    );
    if column_labels.is_none() {
        parts.sort();
    }
    format!("{}  {}", row.symbol, parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldDef;
    use reqwest::header::{ORIGIN, USER_AGENT};
    use serde_json::json;

    #[test]
    fn get_url_joins_subtype() {
        assert_eq!(
            get_url("crypto"),
            "https://scanner.tradingview.com/crypto/scan"
        );
    }

    fn sample_field(label: &str, field_name: &str) -> FieldDef {
        FieldDef {
            label: label.into(),
            field_name: field_name.into(),
            format: None,
            interval: false,
            historical: false,
        }
    }

    #[test]
    fn format_timed_field_rewrites_change_intervals() {
        assert_eq!(format_timed_field("change.15"), "change|15");
        assert_eq!(format_timed_field("change.1W"), "change|1W");
        assert_eq!(format_timed_field("close"), "close");
    }

    #[test]
    fn get_columns_skips_candlestick_and_adds_update_mode() {
        let fields = vec![
            sample_field("Name", "name"),
            sample_field("Pattern", "candlestick_doji"),
            FieldDef {
                label: "Change %".into(),
                field_name: "change".into(),
                format: Some("percent".into()),
                interval: true,
                historical: false,
            },
        ];
        let cols = get_columns_to_request(&fields);
        let keys: Vec<_> = cols.iter().map(|(k, _)| k.as_str()).collect();
        assert!(!keys.iter().any(|k| k.starts_with("candlestick")));
        assert!(keys.contains(&"update_mode"));
    }

    #[test]
    fn get_columns_adds_rec_and_historical() {
        let fields = vec![
            FieldDef {
                label: "RSI".into(),
                field_name: "RSI".into(),
                format: Some("recommendation".into()),
                interval: true,
                historical: true,
            },
            sample_field("Volume", "volume"),
        ];
        let cols = get_columns_to_request(&fields);
        let keys: Vec<_> = cols.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"Rec.RSI"));
        assert!(keys.contains(&"RSI[1]"));
    }

    #[test]
    fn millify_three_decimals() {
        assert_eq!(millify(1_500.0), "1.500K");
        assert_eq!(millify(2_000_000.0), "2.000M");
        assert_eq!(millify(42.0), "42.000");
        assert_eq!(millify(-1_500.0), "-1.500K");
        assert_eq!(millify(0.0), "0.000");
    }

    #[test]
    fn get_recommendation_letters() {
        assert_eq!(get_recommendation(-1.0), "S");
        assert_eq!(get_recommendation(0.0), "N");
        assert_eq!(get_recommendation(1.0), "B");
    }

    #[test]
    fn is_status_code_ok_range() {
        assert!(is_status_code_ok(200));
        assert!(is_status_code_ok(299));
        assert!(!is_status_code_ok(404));
    }

    #[test]
    fn request_headers_values() {
        let h = request_headers();
        assert_eq!(
            h.get(USER_AGENT).unwrap().to_str().unwrap(),
            DEFAULT_USER_AGENT
        );
        assert_eq!(h.get(ORIGIN).unwrap().to_str().unwrap(), TRADINGVIEW_ORIGIN);
        assert_eq!(TRADINGVIEW_REFERER, format!("{TRADINGVIEW_ORIGIN}/"));
        assert_eq!(
            h.get(reqwest::header::REFERER).unwrap().to_str().unwrap(),
            TRADINGVIEW_REFERER
        );
    }

    #[test]
    fn format_value_null_is_dash_dash() {
        assert_eq!(format_value(&json!(null)), "--");
    }

    #[test]
    fn format_rating_arrows() {
        assert_eq!(format_rating(1.0), "↑ B");
        assert_eq!(format_rating(-1.0), "↓ S");
        assert_eq!(format_rating(0.0), "- N");
    }

    #[test]
    fn format_value_millifies_numbers() {
        assert_eq!(format_value(&json!(1500.0)), "1.500K");
    }

    #[test]
    fn format_row_with_column_filter() {
        let mut data = serde_json::Map::new();
        data.insert("Name".into(), json!("BTC"));
        data.insert("Volume".into(), json!(1_500_000.0));
        let row = ScreenerRow {
            symbol: "BINANCE:BTCUSDT".into(),
            data,
        };
        let rendered = format_row(&row, Some(&["Name", "Volume"]));
        assert!(rendered.contains("BINANCE:BTCUSDT"));
        assert!(rendered.contains("Name: BTC"));
        assert!(rendered.contains("Volume: 1.500M"));
    }
}
