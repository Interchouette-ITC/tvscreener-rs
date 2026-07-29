// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Demo for util / presets / display helpers.
//!
//! ```bash
//! cargo run --example manual_test
//! ```
//!
//! Does not call the scanner. For live scans see `example_crypto`.

use reqwest::header::HeaderName;
use tvscreener::field::{get_preset, list_presets, search_fields, Asset};
use tvscreener::util::{format_recommendation, format_row};
use tvscreener::util::{get_url, millify, request_headers};
use tvscreener::ScreenerRow;

fn main() {
    println!("tvscreener-rs manual_test\n");

    println!("1) util");
    println!("   get_url(global) = {}", get_url("global"));
    println!("   millify(1_500_000) = {}", millify(1_500_000.0));
    let headers = request_headers();
    println!(
        "   request_headers keys: {:?}",
        headers.keys().map(HeaderName::as_str).collect::<Vec<_>>()
    );

    println!("\n2) presets / fields");
    let presets = list_presets();
    println!(
        "   list_presets ({}): {:?}",
        presets.len(),
        &presets[..presets.len().min(8)]
    );
    let stock_price = get_preset("stock_price").expect("stock_price");
    println!(
        "   stock_price fields: {:?}",
        stock_price
            .iter()
            .map(|f| f.label.as_str())
            .collect::<Vec<_>>()
    );
    let hits = search_fields(Asset::Crypto, "volume");
    println!("   search_fields(crypto, volume) → {} hits", hits.len());

    println!("\n3) display");
    let mut data = serde_json::Map::new();
    data.insert("Name".into(), serde_json::json!("BTCUSDT"));
    data.insert("Volume".into(), serde_json::json!(2_500_000.0));
    data.insert("Reco".into(), serde_json::json!(null));
    let row = ScreenerRow {
        symbol: "BINANCE:BTCUSDT".into(),
        data,
    };
    println!(
        "   format_row: {}",
        format_row(&row, Some(&["Name", "Volume", "Reco"]))
    );
    println!(
        "   format_recommendation(1): {}",
        format_recommendation(1.0)
    );
    println!(
        "   format_recommendation(-1): {}",
        format_recommendation(-1.0)
    );

    println!(
        "\nDone. Confidence path: `cargo test` then optional live e2e (see docs/MANUAL_TEST_PLAN.md)."
    );
}
