// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for util, fields, filters, payloads, and typed screeners.
//!
//! Default `cargo test` path. Live HTTP is in `e2e_live`.

use serde_json::{json, Value};
use tvscreener::core::bond::BondScreener;
use tvscreener::core::coin::CoinScreener;
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::core::forex::ForexScreener;
use tvscreener::core::futures::FuturesScreener;
use tvscreener::core::stock::StockScreener;
use tvscreener::core::Screener;
use tvscreener::field::index_symbol;
use tvscreener::field::{
    default_bond_fields, default_coin_fields, default_crypto_fields, default_forex_fields,
    default_futures_fields, default_stock_fields, get_preset, list_presets, search_fields, Asset,
    FieldDef, Market,
};
use tvscreener::filter::{ExtraFilter, FieldCondition, Filter, FilterOperator};
use tvscreener::util::{format_recommendation, format_row, format_value};
use tvscreener::util::{
    get_columns_to_request, get_recommendation, get_url, millify, request_headers, SCANNER_ORIGIN,
    TRADINGVIEW_ORIGIN,
};
use tvscreener::{ScreenerRow, TvscreenerError};

// --- Build / util -----------------------------------------------------------------

#[test]
fn util_get_url_all_subtypes() {
    for subtype in ["global", "crypto", "forex", "bond", "futures", "coin"] {
        let url = get_url(subtype);
        assert!(url.starts_with(SCANNER_ORIGIN));
        assert!(url.ends_with(&format!("/{subtype}/scan")));
    }
}

#[test]
fn util_millify_and_headers() {
    assert_eq!(millify(1_500_000.0), "1.500M");
    assert_eq!(millify(-2_000.0), "-2.000K");
    let headers = request_headers();
    assert_eq!(
        headers
            .get(reqwest::header::ORIGIN)
            .unwrap()
            .to_str()
            .unwrap(),
        TRADINGVIEW_ORIGIN
    );
    assert!(headers.get(reqwest::header::USER_AGENT).is_some());
    assert!(headers.get(reqwest::header::CONTENT_TYPE).is_some());
}

#[test]
fn util_recommendation_letters() {
    assert_eq!(get_recommendation(-0.5), "S");
    assert_eq!(get_recommendation(0.0), "N");
    assert_eq!(get_recommendation(1.2), "B");
}

// --- Fields / presets -------------------------------------------------------------

#[test]
fn fields_defaults_non_empty_for_all_assets() {
    assert!(!default_crypto_fields().is_empty());
    assert!(!default_stock_fields().is_empty());
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

#[test]
fn filters_to_json_wire_shape() {
    let f = Filter::new("RSI", FilterOperator::Below, [json!(35)]);
    let obj = f.to_json();
    assert_eq!(obj["left"], "RSI");
    assert_eq!(obj["operation"], "less");
    assert_eq!(obj["right"], 35);
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

#[test]
fn columns_to_request_skips_candlestick_adds_update_mode() {
    let fields = vec![
        FieldDef::new("Name", "name"),
        FieldDef {
            label: "Pattern".into(),
            field_name: "candlestick_doji".into(),
            format: None,
            interval: false,
            historical: false,
        },
        FieldDef {
            label: "RSI".into(),
            field_name: "RSI".into(),
            format: Some("recommendation".into()),
            interval: true,
            historical: true,
        },
    ];
    let cols = get_columns_to_request(&fields);
    assert!(cols.iter().any(|(t, _)| t == "update_mode"));
    assert!(!cols.iter().any(|(t, _)| t.starts_with("candlestick")));
    assert!(cols.iter().any(|(t, _)| t == "Rec.RSI"));
    assert!(cols.iter().any(|(t, _)| t == "RSI[1]"));
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

// --- Beauty -----------------------------------------------------------------------

#[test]
fn display_format_row_and_recommendation() {
    let mut data = serde_json::Map::new();
    data.insert("Name".into(), json!("BTC"));
    data.insert("Volume".into(), json!(2_500_000.0));
    data.insert("Missing".into(), Value::Null);
    let row = ScreenerRow {
        symbol: "BINANCE:BTCUSDT".into(),
        data,
    };
    let rendered = format_row(&row, Some(&["Name", "Volume", "Missing"]));
    assert!(rendered.contains("BINANCE:BTCUSDT"));
    assert!(rendered.contains("Name: BTC"));
    assert!(rendered.contains("Volume: 2.500M"));
    assert!(rendered.contains("Missing: --"));
    assert_eq!(format_recommendation(1.0), "↑ B");
    assert_eq!(format_recommendation(-1.0), "↓ S");
    assert_eq!(format_value(&json!(null)), "--");
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
    assert!(TvscreenerError::Network("x".into())
        .to_string()
        .contains("network"));
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
    let mut s = Screener::new("crypto");
    s.select([FieldDef::new("Name", "name")])
        .set_url("http://127.0.0.1:1/")
        .set_range(0, 1);
    let err = s.get().await.unwrap_err();
    assert!(
        matches!(
            err,
            TvscreenerError::Network(_)
                | TvscreenerError::Timeout
                | TvscreenerError::HttpStatus { .. }
        ),
        "unexpected error: {err:?}"
    );
}

// --- MCP tools (offline) ----------------------------------------------------------

#[cfg(feature = "mcp")]
#[test]
fn mcp_tools_discover_and_presets() {
    use tvscreener::field::Asset;
    use tvscreener::mcp::tools::{
        custom_query_payload_preview, format_discover_fields, format_get_preset,
        format_list_presets, parse_asset, PayloadPreviewOpts,
    };

    assert_eq!(parse_asset("stock").unwrap(), Asset::Stock);
    let text = format_discover_fields(Asset::Crypto, "volume", 8);
    assert!(text.contains("volume") || text.contains("VOLUME") || text.contains("fields"));
    assert!(format_list_presets().contains("crypto_price"));
    assert!(format_get_preset("crypto_price").unwrap().contains("Name"));
    let payload = custom_query_payload_preview(&PayloadPreviewOpts {
        asset_type: "crypto",
        fields: Some("NAME,PRICE"),
        filters: Some(r#"[{"field":"PRICE","op":">","value":1}]"#),
        sort_by: None,
        ascending: false,
        limit: 5,
        indices: None,
        markets: None,
    })
    .unwrap();
    assert_eq!(payload["range"], json!([0, 5]));
}

#[cfg(feature = "mcp")]
#[test]
fn mcp_tools_stock_index_payload_uses_syml_prefix() {
    use tvscreener::mcp::tools::{custom_query_payload_preview, PayloadPreviewOpts};

    let payload = custom_query_payload_preview(&PayloadPreviewOpts {
        asset_type: "stock",
        fields: Some("NAME,PRICE"),
        filters: None,
        sort_by: None,
        ascending: false,
        limit: 5,
        indices: Some("SP500"),
        markets: Some("AMERICA"),
    })
    .unwrap();
    assert_eq!(payload["symbols"]["symbolset"], json!(["SYML:SP;SPX"]));
    assert_eq!(payload["markets"], json!(["america"]));
}
