// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Filter operators, [`Filter`] payloads, [`FieldCondition`], and [`ExtraFilter`].
//!
//! When two filters share the same `left` key,
//! [`crate::core::Screener::add_filter`] merges values and may promote the operator
//! to [`FilterOperator::InRange`].

use serde_json::{json, Value};

/// Comparison operators sent as `"operation"` in `TradingView` filter objects.
///
/// Wire `operation` strings for screener filter objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterOperator {
    /// Wire: `"less"`
    Below,
    /// Wire: `"eless"`
    BelowOrEqual,
    /// Wire: `"greater"`
    Above,
    /// Wire: `"egreater"`
    AboveOrEqual,
    /// Wire: `"crosses"`.
    Crosses,
    /// Wire: `"crosses_above"`
    CrossesUp,
    /// Wire: `"crosses_below"`
    CrossesDown,
    /// Wire: `"in_range"`.
    InRange,
    /// Wire: `"not_in_range"`.
    NotInRange,
    /// Wire: `"equal"`.
    Equal,
    /// Wire: `"nequal"`
    NotEqual,
    /// Wire: `"match"`.
    Match,
}

impl FilterOperator {
    /// `TradingView` scanner JSON `operation` string for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Below => "less",
            Self::BelowOrEqual => "eless",
            Self::Above => "greater",
            Self::AboveOrEqual => "egreater",
            Self::Crosses => "crosses",
            Self::CrossesUp => "crosses_above",
            Self::CrossesDown => "crosses_below",
            Self::InRange => "in_range",
            Self::NotInRange => "not_in_range",
            Self::Equal => "equal",
            Self::NotEqual => "nequal",
            Self::Match => "match",
        }
    }

    /// Parses a wire or friendly `operation` string.
    ///
    /// Accepts `TradingView` wire tokens (`less`, `egreater`, …) and common aliases
    /// (`<`, `>=`, `above`, `not_equal`, …).
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s.trim() {
            "less" | "<" | "below" => Some(Self::Below),
            "eless" | "<=" | "below_or_equal" => Some(Self::BelowOrEqual),
            "greater" | ">" | "above" => Some(Self::Above),
            "egreater" | ">=" | "above_or_equal" => Some(Self::AboveOrEqual),
            "crosses" => Some(Self::Crosses),
            "crosses_above" | "crosses_up" => Some(Self::CrossesUp),
            "crosses_below" | "crosses_down" => Some(Self::CrossesDown),
            "in_range" => Some(Self::InRange),
            "not_in_range" => Some(Self::NotInRange),
            "equal" | "==" | "=" => Some(Self::Equal),
            "nequal" | "!=" | "not_equal" => Some(Self::NotEqual),
            "match" => Some(Self::Match),
            _ => None,
        }
    }
}

impl std::fmt::Display for FilterOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FilterOperator {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_wire(s).ok_or_else(|| format!("unknown filter operation: {s}"))
    }
}

/// Non-field filter targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraFilter {
    /// Wire left key `"active_symbol"`.
    CurrentTradingDay,
    /// Wire left key `"name,description"`.
    Search,
    /// Wire left key `"is_primary"`.
    Primary,
}

impl ExtraFilter {
    /// Payload `left` key for this extra filter.
    #[must_use]
    pub const fn field_name(self) -> &'static str {
        match self {
            Self::CurrentTradingDay => "active_symbol",
            Self::Search => "name,description",
            Self::Primary => "is_primary",
        }
    }
}

/// One scanner filter clause (`left` / `operation` / `right`).
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // `serde_json::Value` is not `Eq`
pub struct Filter {
    /// Column key on the left side of the comparison.
    pub left: String,
    /// Operator applied to `left` vs `values`.
    pub operation: FilterOperator,
    /// Right-hand side value(s); field-to-field compares use column name strings.
    pub values: Vec<Value>,
}

impl Filter {
    /// Builds a filter from one or many right-hand values.
    pub fn new(
        left: impl Into<String>,
        operation: FilterOperator,
        values: impl IntoIterator<Item = Value>,
    ) -> Self {
        Self {
            left: left.into(),
            operation,
            values: values.into_iter().collect(),
        }
    }

    /// Builds a filter on an [`ExtraFilter`] left key.
    pub fn on_extra(
        extra: ExtraFilter,
        operation: FilterOperator,
        values: impl IntoIterator<Item = Value>,
    ) -> Self {
        Self::new(extra.field_name(), operation, values)
    }

    /// Serializes to one `{ "left", "operation", "right" }` object for the scan payload.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let right = match self.values.as_slice() {
            [] => Value::Null,
            [one] => one.clone(),
            many => Value::Array(many.to_vec()),
        };
        json!({
            "left": self.left,
            "operation": self.operation.as_str(),
            "right": right,
        })
    }
}

/// A single comparison before it is merged into the screener filter list
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // `serde_json::Value` is not `Eq`
pub struct FieldCondition {
    /// Left column key (field name).
    pub left: String,
    /// Comparison operator.
    pub operation: FilterOperator,
    /// Right-hand JSON value.
    pub value: Value,
}

impl FieldCondition {
    /// Creates a condition on `left` with the given operator and right-hand value.
    pub fn new(left: impl Into<String>, operation: FilterOperator, value: Value) -> Self {
        Self {
            left: left.into(),
            operation,
            value,
        }
    }

    /// Converts this condition into a [`Filter`]
    #[must_use]
    pub fn into_filter(self) -> Filter {
        Filter::new(self.left, self.operation, [self.value])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_operator_wire_values() {
        assert_eq!(FilterOperator::Below.as_str(), "less");
        assert_eq!(FilterOperator::Match.as_str(), "match");
        assert_eq!(FilterOperator::NotEqual.as_str(), "nequal");
    }

    #[test]
    fn filter_operator_from_wire_aliases() {
        assert_eq!(
            FilterOperator::from_wire(">="),
            Some(FilterOperator::AboveOrEqual)
        );
        assert_eq!(
            FilterOperator::from_wire("above"),
            Some(FilterOperator::Above)
        );
        assert_eq!(
            FilterOperator::from_wire("!="),
            Some(FilterOperator::NotEqual)
        );
        assert_eq!(
            FilterOperator::from_wire("crosses_up"),
            Some(FilterOperator::CrossesUp)
        );
        assert_eq!(FilterOperator::from_wire("nope"), None);
    }

    #[test]
    fn filter_operator_display_from_str_roundtrip() {
        for op in [
            FilterOperator::Below,
            FilterOperator::AboveOrEqual,
            FilterOperator::InRange,
            FilterOperator::Match,
            FilterOperator::NotEqual,
        ] {
            let wire = op.to_string();
            let parsed: FilterOperator = wire.parse().expect("parse");
            assert_eq!(parsed, op);
        }
        assert!("nope".parse::<FilterOperator>().is_err());
    }

    #[test]
    fn extra_filter_field_names() {
        assert_eq!(ExtraFilter::Search.field_name(), "name,description");
        assert_eq!(ExtraFilter::CurrentTradingDay.field_name(), "active_symbol");
        assert_eq!(ExtraFilter::Primary.field_name(), "is_primary");
    }

    #[test]
    fn filter_on_extra_multi_value_json() {
        let f = Filter::on_extra(
            ExtraFilter::Primary,
            FilterOperator::InRange,
            [json!(true), json!(false)],
        );
        let obj = f.to_json();
        assert_eq!(obj["left"], "is_primary");
        assert_eq!(obj["operation"], "in_range");
        assert_eq!(obj["right"], json!([true, false]));
    }

    #[test]
    fn filter_to_json_single_right() {
        let f = Filter::new("close", FilterOperator::Above, [json!(100)]);
        let obj = f.to_json();
        assert_eq!(obj["left"], "close");
        assert_eq!(obj["operation"], "greater");
        assert_eq!(obj["right"], 100);
    }
}
