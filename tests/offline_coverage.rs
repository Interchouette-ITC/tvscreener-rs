// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for util, fields, filters, payloads, and typed screeners.
//!
//! Default `cargo test` path. Network e2e is in `e2e_live`.

use serde_json::json;
use tvscreener::core::bond::BondScreener;
use tvscreener::core::coin::CoinScreener;
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::core::forex::ForexScreener;
use tvscreener::core::futures::FuturesScreener;
use tvscreener::core::stock::StockScreener;
use tvscreener::core::Screener;
use tvscreener::field::index_symbol;
use tvscreener::field::{
    default_bond_fields, default_coin_fields, default_crypto_fields, default_fields,
    default_forex_fields, default_futures_fields, default_stock_fields, get_preset, list_presets,
    search_fields, Asset, FieldDef, Market,
};
use tvscreener::filter::{ExtraFilter, FieldCondition, FilterOperator};
use tvscreener::TvscreenerError;

// --- Fields / presets -------------------------------------------------------------

#[test]
fn fields_defaults_non_empty_for_all_assets() {
    for asset in [
        Asset::Stock,
        Asset::Crypto,
        Asset::Forex,
        Asset::Bond,
        Asset::Futures,
        Asset::Coin,
    ] {
        let via_dispatch = default_fields(asset);
        assert!(!via_dispatch.is_empty(), "{asset:?}");
    }
    assert_eq!(
        default_fields(Asset::Crypto).len(),
        default_crypto_fields().len()
    );
    assert_eq!(
        default_fields(Asset::Stock).len(),
        default_stock_fields().len()
    );
    assert!(!default_forex_fields().is_empty());
    assert!(!default_bond_fields().is_empty());
    assert!(!default_futures_fields().is_empty());
    assert!(!default_coin_fields().is_empty());
}

#[test]
fn fields_search_and_presets() {
    let hits = search_fields(Asset::Stock, "market cap");
    assert!(!hits.is_empty());
    let names = list_presets();
    assert!(names.iter().any(|n| n == "stock_valuation"));
    assert!(names.iter().any(|n| n == "crypto_price"));
    let valuation = get_preset("stock_valuation").expect("preset");
    assert!(!valuation.is_empty());
    assert!(valuation.iter().any(|f| {
        f.field_name.contains("market_cap") || f.label.to_ascii_lowercase().contains("market")
    }));
    assert!(get_preset("no_such_preset").is_err());
}

// --- Filters ----------------------------------------------------------------------

#[test]
fn filters_numeric_string_list_and_search_payload() {
    let mut s = Screener::new("crypto");
    s.select(default_crypto_fields())
        .add_filter("close", FilterOperator::Above, [json!(100)])
        .unwrap()
        .add_filter("name", FilterOperator::Match, [json!("btc")])
        .unwrap()
        .add_filter(
            "exchange",
            FilterOperator::InRange,
            [json!("BINANCE"), json!("COINBASE")],
        )
        .unwrap()
        .search("eth")
        .unwrap();

    let payload = s.build_payload().expect("payload");
    let filters = payload["filter"].as_array().expect("filter array");
    assert!(filters
        .iter()
        .any(|f| f["left"] == "close" && f["operation"] == "greater"));
    assert!(filters
        .iter()
        .any(|f| f["left"] == ExtraFilter::Search.field_name()));
    assert!(filters.iter().any(|f| {
        f["left"] == "exchange" && f["right"].as_array().is_some_and(|a| a.len() == 2)
    }));
}

#[test]
fn filters_remove_and_condition() {
    let mut s = Screener::new("crypto");
    s.select([FieldDef::new("Name", "name")])
        .where_condition(FieldCondition::new(
            "volume",
            FilterOperator::Above,
            json!(1_000_000),
        ))
        .unwrap()
        .add_filter("close", FilterOperator::Below, [json!(50)])
        .unwrap();
    assert_eq!(s.filters().len(), 2);
    s.remove_filter("volume");
    assert_eq!(s.filters().len(), 1);
    assert_eq!(s.filters()[0].left, "close");
}

// --- Base screener payload --------------------------------------------------------

#[test]
fn base_screener_select_range_sort_payload() {
    let mut s = Screener::new("crypto");
    s.select(get_preset("crypto_price").unwrap())
        .set_range(0, 25)
        .sort_by("24h_vol|5", false)
        .set_print_request(true)
        .add_option("lang", json!("en"));
    let payload = s.build_payload().unwrap();
    assert_eq!(payload["range"], json!([0, 25]));
    assert_eq!(payload["sort"]["sortOrder"], json!("desc"));
    assert!(payload["columns"].as_array().unwrap().len() >= 2);
    assert_eq!(payload["options"]["lang"], json!("en"));
}

#[test]
fn base_screener_set_index_merges_symbolset() {
    let mut s = Screener::new("global");
    s.select([FieldDef::new("Name", "name")])
        .set_index([index_symbol::SP500, index_symbol::NASDAQ_100]);
    let payload = s.build_payload().unwrap();
    assert_eq!(
        payload["symbols"]["symbolset"],
        json!([
            format!("SYML:{}", index_symbol::SP500),
            format!("SYML:{}", index_symbol::NASDAQ_100)
        ])
    );
}

#[test]
fn base_screener_empty_fields_is_invalid() {
    let s = Screener::new("crypto");
    match s.build_payload() {
        Err(TvscreenerError::InvalidRequest(_)) => {}
        other => panic!("expected InvalidRequest, got {other:?}"),
    }
}

#[test]
fn base_screener_parse_rows_maps_labels() {
    let columns = vec![
        ("name".into(), "Name".into()),
        ("close".into(), "Price".into()),
    ];
    let body = json!({
        "data": [
            { "s": "BINANCE:BTCUSDT", "d": ["BTCUSDT", 42000.5] },
            { "s": "BINANCE:ETHUSDT", "d": ["ETHUSDT", 2200.0] }
        ]
    });
    let rows = Screener::parse_rows(&body, &columns).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].symbol, "BINANCE:BTCUSDT");
    assert_eq!(rows[0].data.get("Name"), Some(&json!("BTCUSDT")));
    assert_eq!(rows[0].data.get("Price"), Some(&json!(42000.5)));
}

#[test]
fn base_screener_parse_rows_rejects_bad_shape() {
    let columns = vec![("name".into(), "Name".into())];
    let err = Screener::parse_rows(&json!({}), &columns).unwrap_err();
    assert!(matches!(err, TvscreenerError::Json(_)));
}

// --- Six screeners (constructors + payload, no network) ---------------------------

#[test]
fn stock_screener_defaults_markets_and_symbol_types() {
    let mut ss = StockScreener::new();
    assert!(!ss.inner().selected_fields().is_empty());
    let payload = ss.inner().build_payload().unwrap();
    assert_eq!(payload["markets"], json!([Market::america().value]));

    ss.set_markets([Market::america()])
        .set_symbol_types(["COMMON_STOCK"])
        .expect("symbol types")
        .set_range(0, 5);
    ss.inner_mut().set_index([index_symbol::SP500]);
    let payload = ss.inner().build_payload().unwrap();
    assert!(
        !payload["filter"].as_array().unwrap().is_empty(),
        "symbol type filters should be present"
    );
    assert_eq!(
        payload["symbols"]["symbolset"],
        json!([format!("SYML:{}", index_symbol::SP500)])
    );
}

#[test]
fn crypto_screener_price_conversion_misc() {
    let cs = CryptoScreener::new();
    let payload = cs.inner().build_payload().unwrap();
    assert_eq!(payload["price_conversion"], json!({ "to_symbol": false }));
    assert_eq!(payload["sort"]["sortOrder"], json!("desc"));
}

#[test]
fn forex_bond_futures_coin_constructors_build_payload() {
    let fx = ForexScreener::new();
    let fx_payload = fx.inner().build_payload().unwrap();
    assert!(fx_payload.get("columns").is_some());
    assert_eq!(fx_payload["symbols"]["query"]["types"], json!(["forex"]));

    for (name, payload) in [
        ("bond", BondScreener::new().inner().build_payload().unwrap()),
        (
            "futures",
            FuturesScreener::new().inner().build_payload().unwrap(),
        ),
        ("coin", CoinScreener::new().inner().build_payload().unwrap()),
    ] {
        assert!(
            payload["columns"].as_array().is_some_and(|c| !c.is_empty()),
            "{name} must request columns"
        );
        assert_eq!(payload["range"], json!([0, 150]), "{name} default range");
    }

    let coin = CoinScreener::new().inner().build_payload().unwrap();
    assert_eq!(coin["price_conversion"], json!({ "to_symbol": false }));
}

#[test]
fn select_all_reloads_defaults() {
    let mut cs = CryptoScreener::new();
    cs.select([FieldDef::new("Name", "name")]);
    assert_eq!(cs.inner().selected_fields().len(), 1);
    assert!(!cs.inner().is_select_all());
    cs.select_all();
    assert!(cs.inner().is_select_all());
    assert_eq!(
        cs.inner().selected_fields().len(),
        default_crypto_fields().len()
    );
}

#[test]
fn set_symbols_appears_in_payload() {
    let mut s = Screener::new("forex");
    s.select([FieldDef::new("Name", "name")])
        .set_symbols(json!({ "query": { "types": ["forex"] }, "tickers": [] }));
    let payload = s.build_payload().unwrap();
    assert_eq!(payload["symbols"]["query"]["types"], json!(["forex"]));
}

// --- Errors -----------------------------------------------------------------------

#[test]
fn errors_display_variants() {
    let http = TvscreenerError::HttpStatus {
        status: 400,
        body: "bad".into(),
    };
    assert!(http.to_string().contains("400"));
    assert!(TvscreenerError::Timeout.to_string().contains("timed out"));
    assert!(TvscreenerError::Json("x".into())
        .to_string()
        .contains("JSON"));
    assert!(TvscreenerError::InvalidRequest("x".into())
        .to_string()
        .contains("invalid"));
    assert!(TvscreenerError::InvalidFieldName("nope".into())
        .to_string()
        .contains("invalid field name"));
}

#[tokio::test]
async fn errors_unreachable_url_is_network_or_timeout() {
    use std::error::Error;

    let mut s = Screener::new("crypto");
    s.select([FieldDef::new("Name", "name")])
        .set_url("http://127.0.0.1:1/")
        .set_range(0, 1);
    let err = s.get().await.expect_err("connection to :1 should fail");
    match &err {
        TvscreenerError::Network(_) => {
            assert!(
                err.source().is_some(),
                "Network must expose reqwest::Error via Error::source"
            );
        }
        TvscreenerError::Timeout => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

// (MCP payload smoke lives in `src/mcp/tools` unit tests.)
