// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Network e2e against `scanner.tradingview.com`.
//!
//! Run **serially** to avoid `TradingView` rate limits / parallel timeouts:
//!
//! ```bash
//! TVSCREENER_LIVE=1 cargo test --features live --test e2e_live -- --test-threads=1 --nocapture
//! ```

#![cfg(feature = "live")]

use std::time::Duration;
use tvscreener::core::bond::BondScreener;
use tvscreener::core::coin::CoinScreener;
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::core::forex::ForexScreener;
use tvscreener::core::futures::FuturesScreener;
use tvscreener::core::stock::StockScreener;
use tvscreener::field::index_symbol;
use tvscreener::field::{get_preset, Market};
use tvscreener::filter::{FieldCondition, FilterOperator};
use tvscreener::{Result, ScreenerRow, TvscreenerError};

fn require_live_env() {
    assert_eq!(
        std::env::var("TVSCREENER_LIVE").ok().as_deref(),
        Some("1"),
        "set TVSCREENER_LIVE=1 to run live e2e (also requires --features live)"
    );
}

async fn with_retries<F, Fut>(label: &str, mut run: F) -> Vec<ScreenerRow>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<ScreenerRow>>>,
{
    let mut last_err = None;
    for attempt in 1..=3 {
        match run().await {
            Ok(rows) => return rows,
            Err(err) => {
                eprintln!("{label} attempt {attempt}/3 failed: {err}");
                last_err = Some(err);
                tokio::time::sleep(Duration::from_secs(attempt)).await;
            }
        }
    }
    panic!("{label} get failed after retries: {last_err:?}");
}

async fn assert_non_empty_get<F, Fut>(label: &str, run: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<ScreenerRow>>>,
{
    let rows = with_retries(label, run).await;
    assert!(
        !rows.is_empty(),
        "{label}: expected at least one row, got 0"
    );
    assert!(!rows[0].symbol.is_empty(), "{label}: empty symbol");
    assert!(
        !rows[0].data.is_empty(),
        "{label}: row data map should not be empty"
    );
}

#[tokio::test]
async fn live_crypto_get_returns_rows() {
    require_live_env();
    let mut screener = CryptoScreener::new();
    screener
        .select(get_preset("crypto_price").expect("preset"))
        .set_range(0, 3);
    assert_non_empty_get("crypto", || screener.get()).await;
}

#[tokio::test]
async fn live_stock_get_america_market() {
    require_live_env();
    let mut screener = StockScreener::new();
    screener
        .select(get_preset("stock_price").expect("preset"))
        .set_markets([Market::america()])
        .set_range(0, 3);
    assert_non_empty_get("stock", || screener.get()).await;
}

#[tokio::test]
async fn live_stock_set_index_payload_roundtrip() {
    require_live_env();
    let mut screener = StockScreener::new();
    screener
        .select(get_preset("stock_price").expect("preset"))
        .set_markets([Market::america()])
        .set_range(0, 5);
    screener.inner_mut().set_index([index_symbol::SP500]);
    let payload = screener.inner().build_payload().expect("payload");
    assert_eq!(
        payload["symbols"]["symbolset"],
        serde_json::json!([format!("SYML:{}", index_symbol::SP500)])
    );
    assert_non_empty_get("stock+index", || screener.get()).await;
}

#[tokio::test]
async fn live_forex_get_returns_rows() {
    require_live_env();
    let mut screener = ForexScreener::new();
    screener
        .select(get_preset("forex_price").expect("preset"))
        .set_range(0, 3);
    assert_non_empty_get("forex", || screener.get()).await;
}

#[tokio::test]
async fn live_bond_get_returns_rows() {
    require_live_env();
    let mut screener = BondScreener::new();
    screener
        .select(get_preset("bond_basic").expect("preset"))
        .set_range(0, 3);
    assert_non_empty_get("bond", || screener.get()).await;
}

#[tokio::test]
async fn live_futures_get_returns_rows() {
    require_live_env();
    let mut screener = FuturesScreener::new();
    screener
        .select(get_preset("futures_price").expect("preset"))
        .set_range(0, 3);
    assert_non_empty_get("futures", || screener.get()).await;
}

#[tokio::test]
async fn live_coin_get_returns_rows() {
    require_live_env();
    let mut screener = CoinScreener::new();
    screener
        .select(get_preset("coin_price").expect("preset"))
        .set_range(0, 3);
    assert_non_empty_get("coin", || screener.get()).await;
}

#[tokio::test]
async fn live_crypto_stream_two_iterations() {
    require_live_env();
    let mut screener = CryptoScreener::new();
    screener
        .select(get_preset("crypto_price").expect("preset"))
        .set_range(0, 2);
    let batches = screener.inner().stream(1.0, Some(2)).await;
    assert!(
        !batches.is_empty(),
        "expected at least one successful stream batch"
    );
    assert!(!batches[0].is_empty());
}

#[tokio::test]
async fn live_crypto_stream_with_callback() {
    require_live_env();
    let mut screener = CryptoScreener::new();
    screener
        .select(get_preset("crypto_price").expect("preset"))
        .set_range(0, 1);
    let mut hits = 0usize;
    screener
        .inner()
        .stream_with_callback(1.0, Some(2), |rows| {
            if !rows.is_empty() {
                hits += 1;
            }
        })
        .await;
    assert!(
        hits >= 1,
        "stream callback expected at least one non-empty batch"
    );
}

#[tokio::test]
async fn live_crypto_filter_and_search() {
    require_live_env();
    let mut screener = CryptoScreener::new();
    screener
        .select(get_preset("crypto_volume").expect("preset"))
        .search("BTC")
        .unwrap()
        .set_range(0, 2);
    let rows = with_retries("crypto search", || screener.get()).await;
    assert!(!rows.is_empty());
    let joined = rows
        .iter()
        .map(|r| r.symbol.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        joined.contains("BTC")
            || rows.iter().any(|r| {
                r.data
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|n| n.to_ascii_uppercase().contains("BTC"))
            }),
        "expected BTC-related row, got symbols={joined}"
    );
}

#[tokio::test]
async fn live_crypto_rsi_filter() {
    require_live_env();
    let mut screener = CryptoScreener::new();
    let mut fields = get_preset("crypto_technical").expect("preset");
    if !fields.iter().any(|f| f.field_name == "RSI") {
        fields.push(tvscreener::field::crypto_rsi_14());
    }
    screener
        .select(fields)
        .set_range(0, 5)
        .where_condition(FieldCondition::new(
            "RSI",
            FilterOperator::Below,
            serde_json::json!(40),
        ))
        .unwrap();
    let _rows = with_retries("rsi filter", || screener.get()).await;
}

#[tokio::test]
async fn live_feature_compiled() {
    require_live_env();
    let _ = TvscreenerError::Timeout;
}
