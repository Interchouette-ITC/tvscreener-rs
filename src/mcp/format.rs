// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Text-formatting helpers for MCP tool responses.

use crate::error::Result;
use crate::field::{
    all_countries, all_exchanges, all_index_symbols, all_industries, all_markets, all_ratings,
    all_sectors, get_preset, list_presets, resolve_field, search_fields, to_symbolset_wire, Asset,
};
use crate::util::format_value;
use crate::ScreenerRow;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Formats `discover_fields` search hits as text.
#[must_use]
pub fn format_discover_fields(asset: Asset, search_term: &str, limit: usize) -> String {
    let limit = limit.clamp(1, 100);
    let hits = search_fields(asset, search_term);
    let mut lines = Vec::new();
    for def in hits.into_iter().take(limit) {
        let Some((const_name, resolved)) = resolve_field(asset, &def.field_name) else {
            continue;
        };
        let marker = if resolved.interval {
            " [Technical]"
        } else {
            ""
        };
        lines.push(format!("- **{const_name}**: {}{marker}", resolved.label));
    }
    if lines.is_empty() {
        format!("No fields found matching '{search_term}'")
    } else {
        format!(
            "Found {} fields matching '{search_term}':\n\n{}\n",
            lines.len(),
            lines.join("\n")
        )
    }
}

/// Lists sample field categories for an asset (keyword buckets).
#[must_use]
pub fn format_field_types(asset: Asset) -> String {
    const KEYWORDS: &[&str] = &[
        "price",
        "volume",
        "moving_average",
        "rsi",
        "macd",
        "bollinger",
        "earnings",
        "dividend",
        "market_cap",
        "sector",
        "recommend",
    ];
    let mut out = format!("Field categories for {}:\n\n", asset.as_str());
    for keyword in KEYWORDS {
        let names: Vec<String> = search_fields(asset, keyword)
            .into_iter()
            .take(5)
            .filter_map(|def| resolve_field(asset, &def.field_name).map(|(n, _)| n))
            .collect();
        if names.is_empty() {
            continue;
        }
        let _ = writeln!(out, "**{keyword}**:");
        for name in names {
            let _ = writeln!(out, "  - {name}");
        }
        out.push('\n');
    }
    out.push_str("Use discover_fields('keyword') to find more fields in each category.\n");
    out
}

/// Formats preset names as text.
#[must_use]
pub fn format_list_presets() -> String {
    let names = list_presets();
    let mut out = String::from("Available presets:\n");
    for name in names {
        let _ = writeln!(out, "  - {name}");
    }
    out
}

/// Lists stock sectors (`sector::*` const → wire value).
#[must_use]
pub fn format_list_sectors() -> String {
    let mut out = String::from("Available sectors (const → wire):\n");
    for s in all_sectors() {
        let _ = writeln!(out, "  - **{}**: `{}`", s.const_name, s.value);
    }
    out.push_str(
        "\nPass const names (e.g. `TECHNOLOGY_SERVICES`) or wire values to `search_stocks` `sectors` CSV.\n",
    );
    out
}

/// Lists countries (`country::*` const → wire value).
#[must_use]
pub fn format_list_countries() -> String {
    let mut out = String::from("Available countries (const → wire):\n");
    for c in all_countries() {
        let _ = writeln!(out, "  - **{}**: `{}`", c.const_name, c.value);
    }
    out
}

/// Lists industries (`industry::*` const → wire value).
#[must_use]
pub fn format_list_industries() -> String {
    let mut out = String::from("Available industries (const → wire):\n");
    for i in all_industries() {
        let _ = writeln!(out, "  - **{}**: `{}`", i.const_name, i.value);
    }
    out
}

/// Lists exchanges (`exchange::*` const → wire value).
#[must_use]
pub fn format_list_exchanges() -> String {
    let mut out = String::from("Available exchanges (const → wire):\n");
    for e in all_exchanges() {
        let _ = writeln!(out, "  - **{}**: `{}`", e.const_name, e.value);
    }
    out
}

/// Lists recommendation rating bands.
#[must_use]
pub fn format_list_ratings() -> String {
    let mut out = String::from("Available recommendation ratings (const → [min, max] → label):\n");
    for r in all_ratings() {
        let range = r
            .range()
            .map_or_else(|| "n/a".into(), |[lo, hi]| format!("[{lo}, {hi}]"));
        let _ = writeln!(out, "  - **{}**: {} - {}", r.const_name, range, r.label);
    }
    out
}

/// Documents filter operators accepted by `custom_query`.
#[must_use]
pub fn format_list_filter_operators() -> String {
    const OPS: &[(&str, &str)] = &[
        (">=", "Greater than or equal"),
        (">", "Greater than"),
        ("<=", "Less than or equal"),
        ("<", "Less than"),
        ("==", "Equal to"),
        ("!=", "Not equal to"),
        ("match", "Text contains (for text fields like SECTOR)"),
        (
            "in_range",
            "Value between [min, max] (e.g. RSI between 30-70)",
        ),
        ("not_in_range", "Value outside [min, max]"),
        ("crosses", "Value crosses another"),
        ("crosses_up", "Value crosses above another"),
        ("crosses_down", "Value crosses below another"),
    ];
    let mut out = String::from("Available filter operators for custom_query:\n\n");
    for (op, desc) in OPS {
        let _ = writeln!(out, "- **{op}**: {desc}");
    }
    out.push_str(
        "\nExample filters JSON:\n\
         ```json\n\
         [\n\
           {\"field\": \"PRICE\", \"op\": \">=\", \"value\": 100},\n\
           {\"field\": \"RSI\", \"op\": \"in_range\", \"value\": [30, 70]},\n\
           {\"field\": \"SECTOR\", \"op\": \"match\", \"value\": \"Technology Services\"}\n\
         ]\n\
         ```\n",
    );
    out
}

/// Lists stock markets (`Market` const → wire value).
#[must_use]
pub fn format_list_markets() -> String {
    let mut out = String::from("Available markets (const → wire):\n");
    for m in all_markets() {
        let _ = writeln!(out, "  - **{}**: `{}`", m.const_name, m.value);
    }
    out.push_str(
        "\nPass const names or wire values to `build_payload` / `search_by_index` via `markets` CSV.\n",
    );
    out
}

/// Lists index symbolsets for stock `set_index`.
#[must_use]
pub fn format_list_index_symbols() -> String {
    let mut out = String::from("Available index symbols (const → catalog → payload symbolset):\n");
    for idx in all_index_symbols() {
        let wire = to_symbolset_wire(&idx.value);
        let _ = writeln!(
            out,
            "  - **{}**: `{}` → `{}` ({})",
            idx.const_name, idx.value, wire, idx.label
        );
    }
    out.push_str(
        "\nPass const names (e.g. `SP500`) or catalog values (e.g. `SP;SPX`) to `indices` CSV.\n",
    );
    out
}

/// Formats one preset's fields as text.
///
/// # Errors
///
/// Propagates unknown-preset errors from [`get_preset`].
pub fn format_get_preset(name: &str) -> Result<String> {
    let fields = get_preset(name)?;
    let mut out = format!("Preset `{name}` ({} fields):\n\n", fields.len());
    for field in fields {
        let _ = writeln!(out, "- {} (`{}`)", field.label, field.field_name);
    }
    Ok(out)
}

/// Renders rows as a Markdown table (symbol + selected labels).
#[must_use]
pub fn format_rows_markdown(rows: &[ScreenerRow], max_rows: usize) -> String {
    if rows.is_empty() {
        return "No results found matching the criteria.".into();
    }
    let rows: Vec<&ScreenerRow> = rows.iter().take(max_rows).collect();
    let mut labels: Vec<String> = rows
        .iter()
        .flat_map(|r| r.data.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // Prefer a stable short set when many columns
    if labels.len() > 12 {
        labels.truncate(12);
    }

    let mut header = String::from("| Symbol |");
    let mut sep = String::from("| --- |");
    for label in &labels {
        let _ = write!(header, " {label} |");
        sep.push_str(" --- |");
    }
    header.push('\n');
    sep.push('\n');

    let mut body = String::new();
    for row in rows {
        let _ = write!(body, "| {} |", row.symbol);
        for label in &labels {
            let cell = row
                .data
                .get(label)
                .map_or_else(|| "--".into(), format_value);
            let _ = write!(body, " {cell} |");
        }
        body.push('\n');
    }
    format!("{header}{sep}{body}")
}
