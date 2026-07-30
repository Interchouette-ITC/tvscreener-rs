// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Field-aware cell formatting (Python `tvscreener.beauty` without Jupyter HTML).
//!
//! Applies catalog `format` hints: percent coloring, rating labels, millify groups,
//! recommendation arrows, and ADX / AO / Bollinger computed signals.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::field::{rating_find, FieldDef};
use crate::ta::{self, Signal};
use crate::util::{add_rec, format_recommendation, millify};
use crate::ScreenerRow;

/// Semantic color for a formatted cell (map to ANSI / Ratatui later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CellTone {
    /// Default / missing / unstyled.
    #[default]
    Neutral,
    /// Positive percent (`TradingView` green).
    Positive,
    /// Negative percent (`TradingView` red).
    Negative,
    /// Buy / strong-buy recommendation arrow.
    Buy,
    /// Sell / strong-sell recommendation arrow.
    Sell,
}

impl CellTone {
    /// TradingView-ish sRGB triple used by Python CSS (`beauty.py`).
    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Neutral => (157, 178, 189),
            Self::Positive => (0, 169, 127),
            Self::Negative => (255, 23, 62),
            Self::Buy => (41, 98, 255),
            Self::Sell => (255, 74, 104),
        }
    }
}

/// One display cell: plain text plus optional tone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedCell {
    /// Rendered text (`"--"` for missing).
    pub text: String,
    /// Color hint for terminal / TUI.
    pub tone: CellTone,
}

impl FormattedCell {
    /// Neutral cell with the given text.
    #[must_use]
    pub fn neutral(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: CellTone::Neutral,
        }
    }

    /// Missing-value cell (`"--"`).
    #[must_use]
    pub fn missing() -> Self {
        Self::neutral("--")
    }
}

/// Technical `field_name` → raw JSON value for one row (Python uses technical columns).
#[derive(Debug, Clone, Default)]
pub struct RowTechMap {
    map: HashMap<String, Value>,
}

impl RowTechMap {
    /// Builds a map from `(technical_name, label)` pairs and a label-keyed row.
    #[must_use]
    pub fn from_row(row: &ScreenerRow, columns: &[(String, String)]) -> Self {
        let mut map = HashMap::with_capacity(columns.len());
        for (tech, label) in columns {
            if let Some(value) = row.data.get(label) {
                map.insert(tech.clone(), value.clone());
            }
        }
        Self { map }
    }

    /// Inserts a technical → value pair (tests / manual assembly).
    pub fn insert(&mut self, field_name: impl Into<String>, value: Value) {
        self.map.insert(field_name.into(), value);
    }

    /// Looks up a value by technical field name.
    #[must_use]
    pub fn get(&self, field_name: &str) -> Option<&Value> {
        self.map.get(field_name)
    }
}

/// Formats one JSON value using a catalog format hint (no row companions).
///
/// Unhandled formats (`float`, `text`, `date`, …) keep a plain display (no millify).
/// For `recommendation` / `computed_recommendation` without companions, returns the
/// plain value (Python needs the full row for arrows).
#[must_use]
pub fn format_cell(value: &Value, format: Option<&str>) -> FormattedCell {
    match format {
        Some("bool") => format_bool(value),
        Some("rating") => format_rating_label(value),
        Some("round") => format_round(value),
        Some("percent") => format_percent(value),
        Some("currency") => format_currency(value),
        Some("number_group") => format_number_group(value),
        Some("recommendation" | "computed_recommendation") => {
            if is_missing(value) {
                FormattedCell::missing()
            } else {
                FormattedCell::neutral(display_plain(value))
            }
        }
        _ => format_plain(value),
    }
}

/// Formats a cell using [`FieldDef`] metadata and optional technical companions.
///
/// Companion map enables Python parity for `recommendation` (`Rec.{name}`) and for
/// ADX / AO / Bollinger `computed_recommendation` columns.
#[must_use]
pub fn format_cell_for_field(
    value: &Value,
    field: &FieldDef,
    tech: Option<&RowTechMap>,
) -> FormattedCell {
    match field.format.as_deref() {
        Some("recommendation") => format_recommendation_field(value, field, tech),
        Some("computed_recommendation") => format_computed_recommendation(value, field, tech),
        other => format_cell(value, other),
    }
}

fn format_recommendation_field(
    value: &Value,
    field: &FieldDef,
    tech: Option<&RowTechMap>,
) -> FormattedCell {
    if is_missing(value) {
        return FormattedCell::missing();
    }
    let plain = display_plain(value);
    let Some(map) = tech else {
        return FormattedCell::neutral(plain);
    };
    let rec_key = add_rec(&field.field_name);
    let Some(rec) = map.get(&rec_key).and_then(as_finite_f64) else {
        return FormattedCell::neutral(plain);
    };
    let letter = format_recommendation(rec);
    let tone = tone_from_letter(&letter);
    FormattedCell {
        text: format!("{plain} {letter}"),
        tone,
    }
}

fn format_computed_recommendation(
    value: &Value,
    field: &FieldDef,
    tech: Option<&RowTechMap>,
) -> FormattedCell {
    if is_missing(value) {
        return FormattedCell::missing();
    }
    let plain = display_plain(value);
    let Some(map) = tech else {
        return FormattedCell::neutral(plain);
    };
    let signal = match field.field_name.as_str() {
        "ADX" => computed_adx(map),
        "AO" => computed_ao(map),
        "BB.lower" => computed_bb_lower(map, value),
        "BB.upper" => computed_bb_upper(map, value),
        _ => None,
    };
    match signal {
        Some(signal) => FormattedCell {
            text: format!("{plain} {}", signal.letter()),
            tone: tone_from_signal(signal),
        },
        None => FormattedCell::neutral(plain),
    }
}

fn computed_adx(map: &RowTechMap) -> Option<Signal> {
    let adx_v = map.get("ADX").and_then(as_finite_f64)?;
    let dminus = map.get("ADX-DI").and_then(as_finite_f64)?;
    let dplus = map.get("ADX+DI").and_then(as_finite_f64)?;
    let dminus_old = map.get("ADX-DI[1]").and_then(as_finite_f64)?;
    let dplus_old = map.get("ADX+DI[1]").and_then(as_finite_f64)?;
    Some(ta::adx(adx_v, dminus, dplus, dminus_old, dplus_old))
}

fn computed_ao(map: &RowTechMap) -> Option<Signal> {
    let ao_v = map.get("AO").and_then(as_finite_f64)?;
    let ao1 = map.get("AO[1]").and_then(as_finite_f64)?;
    let ao2 = map.get("AO[2]").and_then(as_finite_f64)?;
    Some(ta::ao(ao_v, ao1, ao2))
}

fn computed_bb_lower(map: &RowTechMap, band: &Value) -> Option<Signal> {
    let low = as_finite_f64(band)?;
    let close = map.get("close").and_then(as_finite_f64)?;
    Some(ta::bb_lower(low, close))
}

fn computed_bb_upper(map: &RowTechMap, band: &Value) -> Option<Signal> {
    let up = as_finite_f64(band)?;
    let close = map.get("close").and_then(as_finite_f64)?;
    Some(ta::bb_upper(up, close))
}

fn format_bool(value: &Value) -> FormattedCell {
    if is_missing(value) {
        return FormattedCell::missing();
    }
    let flag = match value {
        Value::Bool(b) => *b,
        Value::String(s) => s == "true",
        _ => false,
    };
    FormattedCell::neutral(flag.to_string())
}

fn format_rating_label(value: &Value) -> FormattedCell {
    let Some(n) = as_finite_f64(value) else {
        return FormattedCell::missing();
    };
    FormattedCell::neutral(rating_find(n).label)
}

fn format_round(value: &Value) -> FormattedCell {
    let Some(n) = as_finite_f64(value) else {
        return FormattedCell::missing();
    };
    FormattedCell::neutral(format!("{:.2}", round2(n)))
}

fn format_percent(value: &Value) -> FormattedCell {
    let Some(n) = as_finite_f64(value) else {
        return FormattedCell::missing();
    };
    let text = format!("{n:.2}%");
    let tone = if text.starts_with('-') {
        CellTone::Negative
    } else {
        CellTone::Positive
    };
    FormattedCell { text, tone }
}

fn format_currency(value: &Value) -> FormattedCell {
    let Some(n) = as_finite_f64(value) else {
        return FormattedCell::missing();
    };
    FormattedCell::neutral(millify(round2(n)))
}

fn format_number_group(value: &Value) -> FormattedCell {
    // Python fillna(0) then millify.
    let n = as_finite_f64(value).unwrap_or(0.0);
    FormattedCell::neutral(millify(n))
}

fn format_plain(value: &Value) -> FormattedCell {
    if is_missing(value) {
        FormattedCell::missing()
    } else {
        FormattedCell::neutral(display_plain(value))
    }
}

fn display_plain(value: &Value) -> String {
    match value {
        Value::Null => "--".into(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn is_missing(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Number(n) => n.as_f64().is_some_and(f64::is_nan),
        _ => false,
    }
}

fn as_finite_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64().filter(|f| f.is_finite()),
        Value::String(s) => s.parse::<f64>().ok().filter(|f| f.is_finite()),
        _ => None,
    }
}

fn round2(n: f64) -> f64 {
    (n * 100.0).round() / 100.0
}

const fn tone_from_signal(signal: Signal) -> CellTone {
    match signal {
        Signal::Buy => CellTone::Buy,
        Signal::Sell => CellTone::Sell,
        Signal::Neutral => CellTone::Neutral,
    }
}

fn tone_from_letter(letter: &str) -> CellTone {
    if letter == Signal::Buy.letter() {
        CellTone::Buy
    } else if letter == Signal::Sell.letter() {
        CellTone::Sell
    } else {
        CellTone::Neutral
    }
}

/// Default max data columns in [`format_rows_table`] (plus the Symbol column).
pub const DEFAULT_TABLE_MAX_COLUMNS: usize = 12;

/// Options for aligned table rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableFormatOptions {
    /// Max field columns (Symbol is always included).
    pub max_columns: usize,
    /// When true, apply truecolor ANSI from [`CellTone::rgb`].
    pub color: bool,
}

impl Default for TableFormatOptions {
    fn default() -> Self {
        Self {
            max_columns: DEFAULT_TABLE_MAX_COLUMNS,
            color: false,
        }
    }
}

/// Renders rows as an aligned text table using field-aware [`format_cell_for_field`].
///
/// Columns follow `fields` order (skipping candlestick), preferring labels present in
/// the rows, truncated to [`TableFormatOptions::max_columns`].
#[must_use]
pub fn format_rows_table(
    rows: &[ScreenerRow],
    fields: &[FieldDef],
    opts: &TableFormatOptions,
) -> String {
    if rows.is_empty() {
        return "(no rows)".into();
    }
    let selected = select_table_fields(rows, fields, opts.max_columns.max(1));
    let columns = crate::util::get_columns_to_request(fields);

    let mut headers: Vec<String> = Vec::with_capacity(selected.len() + 1);
    headers.push("Symbol".into());
    headers.extend(selected.iter().map(|f| f.label.clone()));

    let mut plain_rows: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    let mut tones: Vec<Vec<CellTone>> = Vec::with_capacity(rows.len());

    for row in rows {
        let tech = RowTechMap::from_row(row, &columns);
        let mut plain = Vec::with_capacity(selected.len() + 1);
        let mut row_tones = Vec::with_capacity(selected.len() + 1);
        plain.push(row.symbol.clone());
        row_tones.push(CellTone::Neutral);
        for field in &selected {
            let value = row.data.get(&field.label).unwrap_or(&Value::Null);
            let cell = format_cell_for_field(value, field, Some(&tech));
            plain.push(cell.text);
            row_tones.push(cell.tone);
        }
        plain_rows.push(plain);
        tones.push(row_tones);
    }

    let widths = column_widths(&headers, &plain_rows);
    let mut out = String::new();
    out.push_str(&format_table_line(&headers, &widths, None, false));
    out.push('\n');
    for (plain, row_tones) in plain_rows.iter().zip(tones.iter()) {
        out.push_str(&format_table_line(
            plain,
            &widths,
            Some(row_tones),
            opts.color,
        ));
        out.push('\n');
    }
    out
}

fn select_table_fields<'a>(
    rows: &[ScreenerRow],
    fields: &'a [FieldDef],
    max_columns: usize,
) -> Vec<&'a FieldDef> {
    let present: HashSet<&str> = rows
        .iter()
        .flat_map(|r| r.data.keys().map(String::as_str))
        .collect();

    let mut selected: Vec<&FieldDef> = fields
        .iter()
        .filter(|f| !f.field_name.starts_with("candlestick"))
        .filter(|f| present.contains(f.label.as_str()))
        .take(max_columns)
        .collect();

    if selected.is_empty() {
        selected = fields
            .iter()
            .filter(|f| !f.field_name.starts_with("candlestick"))
            .take(max_columns)
            .collect();
    }
    selected
}

fn column_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = headers.iter().map(String::len).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if let Some(w) = widths.get_mut(i) {
                *w = (*w).max(cell.len());
            }
        }
    }
    widths
}

fn format_table_line(
    cells: &[String],
    widths: &[usize],
    tones: Option<&[CellTone]>,
    color: bool,
) -> String {
    let mut parts = Vec::with_capacity(cells.len());
    for (i, cell) in cells.iter().enumerate() {
        let width = widths.get(i).copied().unwrap_or(cell.len());
        let padded = format!("{cell:<width$}");
        let tone = tones.and_then(|t| t.get(i)).copied().unwrap_or_default();
        parts.push(paint_cell(&padded, tone, color));
    }
    parts.join("  ")
}

fn paint_cell(text: &str, tone: CellTone, color: bool) -> String {
    if !color || matches!(tone, CellTone::Neutral) {
        return text.to_string();
    }
    let (r, g, b) = tone.rgb();
    format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn field(name: &str, format: &str) -> FieldDef {
        FieldDef {
            label: name.into(),
            field_name: name.into(),
            format: Some(format.into()),
            interval: false,
            historical: false,
        }
    }

    #[test]
    fn percent_formats_and_tones() {
        let up = format_cell(&json!(1.234), Some("percent"));
        assert_eq!(up.text, "1.23%");
        assert_eq!(up.tone, CellTone::Positive);

        let down = format_cell(&json!(-2.5), Some("percent"));
        assert_eq!(down.text, "-2.50%");
        assert_eq!(down.tone, CellTone::Negative);

        assert_eq!(
            format_cell(&json!(null), Some("percent")),
            FormattedCell::missing()
        );
    }

    #[test]
    fn rating_uses_band_labels() {
        assert_eq!(format_cell(&json!(0.75), Some("rating")).text, "Strong Buy");
        assert_eq!(format_cell(&json!(0.0), Some("rating")).text, "Neutral");
        assert_eq!(
            format_cell(&json!(-0.75), Some("rating")).text,
            "Strong Sell"
        );
    }

    #[test]
    fn currency_rounds_then_millifies() {
        assert_eq!(format_cell(&json!(1500.0), Some("currency")).text, "1.500K");
        assert_eq!(
            format_cell(&json!(150.256), Some("currency")).text,
            "150.260"
        );
    }

    #[test]
    fn number_group_null_becomes_zero() {
        assert_eq!(
            format_cell(&json!(null), Some("number_group")).text,
            "0.000"
        );
    }

    #[test]
    fn round_and_bool() {
        assert_eq!(format_cell(&json!(1.239), Some("round")).text, "1.24");
        assert_eq!(format_cell(&json!("true"), Some("bool")).text, "true");
        assert_eq!(format_cell(&json!("false"), Some("bool")).text, "false");
        assert_eq!(format_cell(&json!(true), Some("bool")).text, "true");
    }

    #[test]
    fn float_stays_plain_not_millified() {
        let cell = format_cell(&json!(1500.5), Some("float"));
        assert_eq!(cell.text, "1500.5");
        assert_eq!(cell.tone, CellTone::Neutral);
    }

    #[test]
    fn recommendation_appends_rec_companion() {
        let f = field("RSI", "recommendation");
        let mut map = RowTechMap::default();
        map.insert("RSI", json!(55.2));
        map.insert("Rec.RSI", json!(1.0));
        let cell = format_cell_for_field(&json!(55.2), &f, Some(&map));
        assert_eq!(cell.text, "55.2 ↑ B");
        assert_eq!(cell.tone, CellTone::Buy);

        map.insert("Rec.RSI", json!(-1.0));
        let cell = format_cell_for_field(&json!(55.2), &f, Some(&map));
        assert_eq!(cell.text, "55.2 ↓ S");
        assert_eq!(cell.tone, CellTone::Sell);
    }

    #[test]
    fn computed_bb_lower_buy_when_close_below() {
        let f = field("BB.lower", "computed_recommendation");
        let mut map = RowTechMap::default();
        map.insert("BB.lower", json!(100.0));
        map.insert("close", json!(99.0));
        let cell = format_cell_for_field(&json!(100.0), &f, Some(&map));
        assert_eq!(cell.text, "100.0 ↑ B");
        assert_eq!(cell.tone, CellTone::Buy);
    }

    #[test]
    fn computed_adx_buy_on_cross() {
        let f = field("ADX", "computed_recommendation");
        let mut map = RowTechMap::default();
        map.insert("ADX", json!(25.0));
        map.insert("ADX-DI", json!(10.0));
        map.insert("ADX+DI", json!(15.0));
        map.insert("ADX-DI[1]", json!(12.0));
        map.insert("ADX+DI[1]", json!(8.0));
        let cell = format_cell_for_field(&json!(25.0), &f, Some(&map));
        assert_eq!(cell.text, "25.0 ↑ B");
        assert_eq!(cell.tone, CellTone::Buy);
    }

    #[test]
    fn computed_unknown_indicator_stays_plain() {
        let f = field("RSI", "computed_recommendation");
        let cell = format_cell_for_field(&json!(55.0), &f, None);
        assert_eq!(cell.text, "55.0");
        assert_eq!(cell.tone, CellTone::Neutral);
    }

    #[test]
    fn row_tech_map_from_labels() {
        let mut data = serde_json::Map::new();
        data.insert("Price".into(), json!(42.0));
        data.insert("Reco. RSI".into(), json!(1.0));
        let row = ScreenerRow {
            symbol: "X".into(),
            data,
        };
        let cols = vec![
            ("close".into(), "Price".into()),
            ("Rec.RSI".into(), "Reco. RSI".into()),
        ];
        let map = RowTechMap::from_row(&row, &cols);
        assert_eq!(map.get("close"), Some(&json!(42.0)));
        assert_eq!(map.get("Rec.RSI"), Some(&json!(1.0)));
    }

    #[test]
    fn format_rows_table_aligns_and_beautifies() {
        let fields = vec![
            FieldDef {
                label: "Change %".into(),
                field_name: "change".into(),
                format: Some("percent".into()),
                interval: true,
                historical: false,
            },
            FieldDef {
                label: "Volume".into(),
                field_name: "volume".into(),
                format: Some("number_group".into()),
                interval: false,
                historical: false,
            },
        ];
        let mut data = serde_json::Map::new();
        data.insert("Change %".into(), json!(-1.5));
        data.insert("Volume".into(), json!(1_500_000.0));
        let row = ScreenerRow {
            symbol: "X".into(),
            data,
        };
        let plain = format_rows_table(
            std::slice::from_ref(&row),
            &fields,
            &TableFormatOptions {
                max_columns: 12,
                color: false,
            },
        );
        assert!(plain.contains("Symbol"));
        assert!(plain.contains("Change %"));
        assert!(plain.contains("-1.50%"));
        assert!(plain.contains("1.500M"));

        let colored = format_rows_table(
            &[row],
            &fields,
            &TableFormatOptions {
                max_columns: 12,
                color: true,
            },
        );
        assert!(colored.contains("\x1b[38;2;255;23;62m"));
    }

    #[test]
    fn cell_tone_rgb_matches_python() {
        assert_eq!(CellTone::Positive.rgb(), (0, 169, 127));
        assert_eq!(CellTone::Negative.rgb(), (255, 23, 62));
        assert_eq!(CellTone::Buy.rgb(), (41, 98, 255));
        assert_eq!(CellTone::Sell.rgb(), (255, 74, 104));
        assert_eq!(CellTone::Neutral.rgb(), (157, 178, 189));
    }
}
