// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Example crypto scan - live POST to `TradingView` scanner.
//!
//! Run: `cargo run --example example_crypto`

use tvscreener::core::crypto::CryptoScreener;
use tvscreener::field::{crypto_price, crypto_rsi_14, get_preset};
use tvscreener::filter::{FieldCondition, FilterOperator};
use tvscreener::util::format_value;
use tvscreener::Result;

fn print_row(index: usize, row: &tvscreener::ScreenerRow, labels: &[&str]) {
    println!("  {}: {}", index, row.symbol);
    for label in labels {
        if let Some(value) = row.data.get(*label) {
            println!("      {label}: {}", format_value(value));
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("tvscreener-rs: Crypto presets & CryptoScreener");
    println!("================================================\n");

    println!("1. crypto_price preset (name, price, change %), range=0..2\n");
    {
        let mut screener = CryptoScreener::new();
        let mut fields = get_preset("crypto_price")?;
        if !fields.iter().any(|f| f.label == "Price") {
            fields.push(crypto_price());
        }
        screener.select(fields).set_range(0, 2);
        let rows = screener.get().await?;
        for (index, row) in rows.iter().enumerate() {
            print_row(index + 1, row, &["Name", "Price", "Change %"]);
        }
        println!();
    }

    println!("2. crypto_volume preset, search='BTC', range=0..2\n");
    {
        let mut screener = CryptoScreener::new();
        screener
            .select(get_preset("crypto_volume")?)
            .search("BTC")?
            .set_range(0, 2);
        let rows = screener.get().await?;
        for (index, row) in rows.iter().enumerate() {
            print_row(
                index + 1,
                row,
                &["Name", "Volume", "Volume 24h in USD", "Relative Volume"],
            );
        }
        println!();
    }

    println!("3. crypto_performance preset, range=0..2\n");
    {
        let mut screener = CryptoScreener::new();
        screener
            .select(get_preset("crypto_performance")?)
            .set_range(0, 2);
        let rows = screener.get().await?;
        if let Some(first) = rows.first() {
            let mut columns: Vec<String> = first.data.keys().cloned().collect();
            columns.sort();
            println!("   Got {} row(s). Columns: {columns:?}", rows.len());
            let mut sample_labels: Vec<String> = first.data.keys().cloned().collect();
            sample_labels.sort();
            println!("   Sample ({}): {sample_labels:?}", first.symbol);
        } else {
            println!("   Got 0 row(s).");
        }
        println!();
    }

    println!("4. crypto_technical preset (RSI < 35 oversold), range=0..3\n");
    {
        let mut screener = CryptoScreener::new();
        let rsi = crypto_rsi_14();
        screener
            .select(get_preset("crypto_technical")?)
            .where_condition(FieldCondition::new(
                rsi.field_name,
                FilterOperator::Below,
                serde_json::json!(35),
            ))?
            .set_range(0, 3);
        let rows = screener.get().await?;
        for (index, row) in rows.iter().enumerate() {
            print_row(
                index + 1,
                row,
                &["Relative Strength Index (14)", "MACD Level (12, 26)"],
            );
        }
        println!();
    }

    println!("5. crypto_price with search='Ethereum', range=0..2\n");
    {
        let mut screener = CryptoScreener::new();
        screener
            .select(get_preset("crypto_price")?)
            .search("Ethereum")?
            .set_range(0, 2);
        let rows = screener.get().await?;
        println!("   Got {} row(s).", rows.len());
        for row in &rows {
            let change = row.data.get("Change %");
            println!("   {}: {change:?}", row.symbol);
        }
    }

    Ok(())
}
