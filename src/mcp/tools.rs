// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Offline-friendly helpers shared by MCP tools (no rmcp dependency).

pub use super::format::{
    format_discover_fields, format_field_types, format_get_preset, format_list_countries,
    format_list_exchanges, format_list_filter_operators, format_list_index_symbols,
    format_list_industries, format_list_markets, format_list_presets, format_list_ratings,
    format_list_sectors, format_rows_markdown,
};
pub use super::query::{
    build_payload, custom_query, custom_query_payload_preview, get_top_movers, search_by_index,
    search_crypto, search_forex, search_stocks, BuildPayloadOpts, CustomQueryOpts,
    PayloadPreviewOpts, SearchStocksOpts,
};
pub use super::resolve::{
    parse_asset, parse_f64_range, resolve_index_wires, resolve_market_wires, resolve_sector_wires,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Asset;
    use crate::filter::FilterOperator;

    #[test]
    fn parse_ops_and_asset() {
        assert_eq!(
            FilterOperator::from_wire(">="),
            Some(FilterOperator::AboveOrEqual)
        );
        assert_eq!(
            FilterOperator::from_wire("in_range"),
            Some(FilterOperator::InRange)
        );
        assert_eq!(parse_asset("crypto").unwrap(), Asset::Crypto);
        assert!(parse_asset("nope").is_err());
    }

    #[test]
    fn discover_and_presets_offline() {
        let text = format_discover_fields(Asset::Crypto, "rsi", 5);
        assert!(text.contains("RSI") || text.contains("fields matching"));
        assert!(format_list_presets().contains("crypto_price"));
        assert!(format_get_preset("crypto_price").unwrap().contains("Name"));
        assert!(format_field_types(Asset::Stock).contains("**price**"));
        assert!(format_list_sectors().contains("TECHNOLOGY_SERVICES"));
        assert!(format_list_sectors().contains("Technology Services"));
        assert!(format_list_countries().contains("UNITED_STATES"));
        assert!(format_list_industries().contains("SEMICONDUCTORS"));
        assert!(format_list_exchanges().contains("NASDAQ"));
        assert!(format_list_ratings().contains("STRONG_BUY"));
        assert!(format_list_filter_operators().contains("in_range"));
        assert!(format_list_filter_operators().contains("PRICE"));
        assert!(format_list_filter_operators().contains("Technology Services"));
        assert!(
            format_list_markets().contains("AMERICA") || format_list_markets().contains("america")
        );
        assert!(format_list_index_symbols().contains("SP500"));
        assert!(format_list_index_symbols().contains("SP;SPX"));
    }

    #[test]
    fn custom_payload_preview_merges_filters() {
        let payload = custom_query_payload_preview(&PayloadPreviewOpts {
            asset_type: "stock",
            fields: Some("NAME,PRICE"),
            filters: Some(
                r#"[{"field":"PRICE","op":">","value":50},{"field":"RSI","op":"in_range","value":[30,70]}]"#,
            ),
            sort_by: Some("PRICE"),
            ascending: false,
            limit: 10,
            indices: None,
            markets: None,
        })
        .unwrap();
        assert!(payload["columns"].as_array().unwrap().len() >= 2);
        assert_eq!(payload["range"], serde_json::json!([0, 10]));
        let filters = payload["filter"].as_array().unwrap();
        assert!(filters
            .iter()
            .any(|f| f["left"] == "close" || f["operation"] == "greater"));
    }

    #[test]
    fn build_payload_with_index_and_markets() {
        let text = build_payload(&BuildPayloadOpts {
            asset_type: "stock",
            fields: Some("NAME,PRICE"),
            filters: None,
            sort_by: Some("MARKET_CAPITALIZATION"),
            indices: Some("SP500,NASDAQ_100"),
            markets: Some("AMERICA"),
        })
        .unwrap();
        assert!(text.contains("SYML:SP;SPX"));
        assert!(text.contains("SYML:NASDAQ;NDX"));
        assert!(text.contains("america"));
        assert_eq!(resolve_index_wires(Some("SP;SPX")).unwrap(), vec!["SP;SPX"]);
        assert_eq!(
            resolve_index_wires(Some("SYML:SP;SPX")).unwrap(),
            vec!["SP;SPX"]
        );
        assert_eq!(resolve_index_wires(Some("SP500")).unwrap(), vec!["SP;SPX"]);
        assert_eq!(
            resolve_market_wires(Some("america")).unwrap(),
            vec!["america"]
        );
        assert!(resolve_index_wires(Some("NOPE")).is_err());
    }

    #[test]
    fn payload_preview_indices_emit_syml_symbolset() {
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
        assert_eq!(
            payload["symbols"]["symbolset"],
            serde_json::json!(["SYML:SP;SPX"])
        );
        assert_eq!(payload["markets"], serde_json::json!(["america"]));
    }

    #[test]
    fn format_rows_markdown_empty() {
        assert!(format_rows_markdown(&[], 5).contains("No results"));
    }
}
