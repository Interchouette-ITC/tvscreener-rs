// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Shared filter/sort parsing and application for CLI, TUI, and MCP.

use crate::core::Screener;
use crate::error::{Result, TvscreenerError};
use crate::field::{resolve_field, Asset, FieldDef};
use crate::filter::{FieldCondition, FilterOperator};
use serde_json::{json, Value};

/// Resolves a field const / technical / label name for `asset`.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when the name is unknown for the asset.
pub fn require_resolved(asset: Asset, name: &str) -> Result<FieldDef> {
    resolve_field(asset, name)
        .map(|(_, def)| def)
        .ok_or_else(|| {
            TvscreenerError::InvalidRequest(format!(
                "unknown field `{name}` for asset `{}`",
                asset.as_str()
            ))
        })
}

/// Parses a filters JSON string into a list of `{field, op, value}` objects.
///
/// Accepts a JSON array or a single object.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when the JSON is invalid or the wrong shape.
pub fn parse_filters_json(s: &str) -> Result<Vec<Value>> {
    let raw = s.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
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

/// Parses optional filters JSON (`None` / empty → empty list).
///
/// # Errors
///
/// Same as [`parse_filters_json`].
pub fn parse_filters_arg(filters: Option<&str>) -> Result<Vec<Value>> {
    let Some(raw) = filters.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };
    parse_filters_json(raw)
}

/// Parses a CLI `FIELD:OP:VALUE` token into a [`FieldCondition`].
///
/// Splits on the first two `:` characters. Value is parsed as `f64`, then JSON, else a string.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when the token shape or operator is invalid.
pub fn parse_filter_token(s: &str) -> Result<FieldCondition> {
    let trimmed = s.trim();
    let (field, rest) = trimmed.split_once(':').ok_or_else(|| {
        TvscreenerError::InvalidRequest(format!("filter token `{trimmed}` must be FIELD:OP:VALUE"))
    })?;
    let (op_str, value_raw) = rest.split_once(':').ok_or_else(|| {
        TvscreenerError::InvalidRequest(format!("filter token `{trimmed}` must be FIELD:OP:VALUE"))
    })?;
    let field = field.trim();
    let op_str = op_str.trim();
    if field.is_empty() || op_str.is_empty() {
        return Err(TvscreenerError::InvalidRequest(format!(
            "filter token `{trimmed}` must be FIELD:OP:VALUE"
        )));
    }
    let op = FilterOperator::from_wire(op_str)
        .ok_or_else(|| TvscreenerError::InvalidRequest(format!("unknown filter op `{op_str}`")))?;
    Ok(FieldCondition::new(
        field,
        op,
        parse_filter_value(value_raw),
    ))
}

/// Builds a [`FieldCondition`] from a JSON `{field, op, value}` object.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for missing keys or unknown operators.
pub fn filter_condition_from_json(entry: &Value) -> Result<FieldCondition> {
    let field_name = entry
        .get("field")
        .and_then(Value::as_str)
        .ok_or_else(|| TvscreenerError::InvalidRequest("filter missing field".into()))?;
    let op_str = entry.get("op").and_then(Value::as_str).unwrap_or(">=");
    let op = FilterOperator::from_wire(op_str)
        .ok_or_else(|| TvscreenerError::InvalidRequest(format!("unknown filter op `{op_str}`")))?;
    let value = entry
        .get("value")
        .cloned()
        .ok_or_else(|| TvscreenerError::InvalidRequest("filter missing value".into()))?;
    Ok(FieldCondition::new(field_name, op, value))
}

/// Parses repeatable `--filter` tokens plus optional `--filters` JSON.
///
/// # Errors
///
/// Propagates token, JSON, or operator parse failures.
pub fn parse_filter_conditions_from_cli(
    tokens: &[impl AsRef<str>],
    filters_json: Option<&str>,
) -> Result<Vec<FieldCondition>> {
    let mut out = Vec::new();
    for token in tokens {
        out.push(parse_filter_token(token.as_ref())?);
    }
    for entry in parse_filters_arg(filters_json)? {
        out.push(filter_condition_from_json(&entry)?);
    }
    Ok(out)
}

/// Parses a filter right-hand value: `f64`, then JSON, else a string.
#[must_use]
pub fn parse_filter_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        return json!(n);
    }
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return v;
    }
    json!(trimmed)
}

/// Applies JSON filter objects (`{field, op, value}`) to `screener`.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] for missing keys, unknown fields, or ops.
pub fn apply_filters_json(screener: &mut Screener, asset: Asset, filters: &[Value]) -> Result<()> {
    for entry in filters {
        let condition = filter_condition_from_json(entry)?;
        let def = require_resolved(asset, &condition.left)?;
        screener.where_condition(FieldCondition::new(
            def.field_name,
            condition.operation,
            condition.value,
        ))?;
    }
    Ok(())
}

/// Applies resolved [`FieldCondition`]s (field left may be const / technical / label).
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when a field name cannot be resolved.
pub fn apply_filters(
    screener: &mut Screener,
    asset: Asset,
    conditions: &[FieldCondition],
) -> Result<()> {
    for condition in conditions {
        let def = require_resolved(asset, &condition.left)?;
        screener.where_condition(FieldCondition::new(
            def.field_name,
            condition.operation,
            condition.value.clone(),
        ))?;
    }
    Ok(())
}

/// Sets sort column when `sort_by` is present and non-empty.
///
/// # Errors
///
/// Returns [`TvscreenerError::InvalidRequest`] when the sort field is unknown.
pub fn apply_sort(
    screener: &mut Screener,
    asset: Asset,
    sort_by: Option<&str>,
    ascending: bool,
) -> Result<()> {
    if let Some(name) = sort_by.map(str::trim).filter(|s| !s.is_empty()) {
        let def = require_resolved(asset, name)?;
        screener.sort_by_field(&def, ascending);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterOperator;

    #[test]
    fn parse_filter_token_numeric_and_aliases() {
        let c = parse_filter_token("close:greater:100").expect("token");
        assert_eq!(c.left, "close");
        assert_eq!(c.operation, FilterOperator::Above);
        assert_eq!(c.value, json!(100.0));

        let c = parse_filter_token("VOLUME:>=:1e6").expect("alias");
        assert_eq!(c.left, "VOLUME");
        assert_eq!(c.operation, FilterOperator::AboveOrEqual);
        assert_eq!(c.value, json!(1e6));
    }

    #[test]
    fn parse_filter_token_string_and_json_value() {
        let c = parse_filter_token("name:match:btc").expect("string");
        assert_eq!(c.value, json!("btc"));

        let c = parse_filter_token(r#"name:match:"btc""#).expect("json string");
        assert_eq!(c.value, json!("btc"));

        let c = parse_filter_token("type:in_range:[\"stock\",\"fund\"]").expect("array");
        assert!(c.value.is_array());
    }

    #[test]
    fn parse_filter_token_rejects_bad_shape() {
        assert!(parse_filter_token("close").is_err());
        assert!(parse_filter_token("close:greater").is_err());
        assert!(parse_filter_token(":greater:1").is_err());
        assert!(parse_filter_token("close:nope:1").is_err());
    }

    #[test]
    fn parse_filters_json_array_and_object() {
        let list = parse_filters_json(r#"[{"field":"close","op":">","value":100}]"#).expect("arr");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["field"], "close");

        let list = parse_filters_json(r#"{"field":"close","op":">","value":100}"#).expect("obj");
        assert_eq!(list.len(), 1);

        assert_eq!(parse_filters_arg(None).unwrap().len(), 0);
        assert_eq!(parse_filters_arg(Some("  ")).unwrap().len(), 0);
        assert!(parse_filters_json("true").is_err());
    }

    #[test]
    fn apply_sort_resolves_const_and_technical() {
        let name = require_resolved(Asset::Stock, "NAME").expect("NAME");
        let mut s = Screener::new("stock");
        s.select([name.clone()]);
        apply_sort(&mut s, Asset::Stock, Some("VOLUME"), false).expect("const");
        let payload = s.build_payload().expect("payload");
        assert_eq!(payload["sort"]["sortBy"], "volume");
        assert_eq!(payload["sort"]["sortOrder"], "desc");

        let mut s = Screener::new("stock");
        s.select([name]);
        apply_sort(&mut s, Asset::Stock, Some("volume"), true).expect("technical");
        let payload = s.build_payload().expect("payload");
        assert_eq!(payload["sort"]["sortBy"], "volume");
        assert_eq!(payload["sort"]["sortOrder"], "asc");
    }

    #[test]
    fn apply_filters_token_and_json() {
        let name = require_resolved(Asset::Stock, "NAME").expect("NAME");
        let mut s = Screener::new("stock");
        s.select([name.clone()]);
        let cond = parse_filter_token("close:greater:100").expect("token");
        apply_filters(&mut s, Asset::Stock, &[cond]).expect("apply");
        let payload = s.build_payload().expect("payload");
        let filters = payload["filter"].as_array().expect("filters");
        assert!(filters
            .iter()
            .any(|f| f["left"] == "close" && f["operation"] == "greater"));

        let mut s = Screener::new("stock");
        s.select([name]);
        let list = parse_filters_json(r#"[{"field":"close","op":">","value":100}]"#).expect("json");
        apply_filters_json(&mut s, Asset::Stock, &list).expect("apply json");
        let payload = s.build_payload().expect("payload");
        let filters = payload["filter"].as_array().expect("filters");
        assert!(filters
            .iter()
            .any(|f| f["left"] == "close" && f["operation"] == "greater"));
    }
}
