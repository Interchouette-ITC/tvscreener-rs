// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Offline golden payloads for stock markets / index (`SYML:`) / symbol types.
//!
//! No network - asserts `build_payload` wire shape only.

use serde_json::json;
use tvscreener::core::stock::StockScreener;
use tvscreener::core::Screener;
use tvscreener::field::index_symbol;
use tvscreener::field::{FieldDef, Market};
use tvscreener::filter::FilterOperator;

#[test]
fn golden_stock_markets_america() {
    let mut s = Screener::new("global");
    s.select([FieldDef::new("Name", "name")])
        .set_markets(["america"])
        .set_range(0, 25);
    let payload = s.build_payload().expect("payload");
    assert_eq!(payload["markets"], json!(["america"]));
    assert_eq!(payload["range"], json!([0, 25]));
    assert_eq!(payload["options"]["lang"], json!("en"));
}

#[test]
fn golden_stock_set_index_sp500() {
    let mut s = Screener::new("global");
    s.select([FieldDef::new("Name", "name")])
        .set_index([index_symbol::SP500])
        .set_range(0, 10);
    let payload = s.build_payload().expect("payload");
    assert_eq!(payload["symbols"], json!({ "symbolset": ["SYML:SP;SPX"] }));
}

#[test]
fn golden_stock_set_index_multi() {
    let mut s = Screener::new("global");
    s.select([FieldDef::new("Name", "name")])
        .set_index([index_symbol::SP500, index_symbol::NASDAQ_100]);
    let payload = s.build_payload().expect("payload");
    assert_eq!(
        payload["symbols"]["symbolset"],
        json!(["SYML:SP;SPX", "SYML:NASDAQ;NDX"])
    );
}

#[test]
fn golden_stock_set_index_syml_idempotent() {
    let mut s = Screener::new("global");
    s.select([FieldDef::new("Name", "name")])
        .set_index(["SYML:SP;SPX"]);
    let payload = s.build_payload().expect("payload");
    assert_eq!(payload["symbols"], json!({ "symbolset": ["SYML:SP;SPX"] }));
}

#[test]
fn golden_stock_screener_symbol_types_etf() {
    let mut ss = StockScreener::new();
    ss.set_symbol_types(["ETF"]).expect("symbol types");
    ss.set_range(0, 5);
    let payload = ss.inner().build_payload().expect("payload");
    assert_eq!(payload["markets"], json!([Market::america().value]));
    let filters = payload["filter"].as_array().expect("filters");
    assert!(filters.iter().any(|f| {
        f["left"] == "type" && (f["operation"] == "equal" || f["operation"] == "in_range")
    }));
    assert!(filters.iter().any(|f| f["left"] == "subtype"));
}

#[test]
fn golden_filter_merge_promotes_in_range() {
    let mut s = Screener::new("global");
    s.select([FieldDef::new("Name", "name")])
        .add_filter("type", FilterOperator::Equal, [json!("stock")])
        .unwrap()
        .add_filter("type", FilterOperator::Equal, [json!("fund")])
        .unwrap();
    let payload = s.build_payload().expect("payload");
    let type_filter = payload["filter"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["left"] == "type")
        .expect("type filter");
    assert_eq!(type_filter["operation"], "in_range");
    assert_eq!(type_filter["right"], json!(["stock", "fund"]));
}
