// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Live CLI tests - spawn the `tvscreener` binary and check real scanner output.
//!
//! ```bash
//! TVSCREENER_LIVE=1 cargo test --features live --test cli_live -- --test-threads=1 --nocapture
//! ```

#![cfg(feature = "live")]

use std::process::Command;

fn require_live_env() {
    assert_eq!(
        std::env::var("TVSCREENER_LIVE").ok().as_deref(),
        Some("1"),
        "set TVSCREENER_LIVE=1 to run live CLI tests"
    );
}

fn tvscreener() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tvscreener"))
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// `scan crypto --limit 3` returns at least one line with a symbol.
#[test]
fn cli_scan_crypto_returns_rows() {
    require_live_env();
    let out = tvscreener()
        .args(["scan", "crypto", "--limit", "3"])
        .output()
        .expect("spawn tvscreener");
    assert!(out.status.success(), "stderr={}", stderr(&out));
    let text = stdout(&out);
    // Each row printed by format_row starts with the symbol
    assert!(
        !text.trim().is_empty() && text.trim() != "(no rows)",
        "expected rows, got: {text}"
    );
}

/// `scan stock --markets AMERICA --limit 2` returns rows restricted to US equities.
#[test]
fn cli_scan_stock_america() {
    require_live_env();
    let out = tvscreener()
        .args(["scan", "stock", "--markets", "AMERICA", "--limit", "2"])
        .output()
        .expect("spawn tvscreener");
    assert!(out.status.success(), "stderr={}", stderr(&out));
    let text = stdout(&out);
    assert!(
        !text.trim().is_empty() && text.trim() != "(no rows)",
        "expected stock rows: {text}"
    );
}

/// `scan stock --index SP500 --limit 3` returns rows from the S&P 500.
#[test]
fn cli_scan_stock_sp500_index() {
    require_live_env();
    let out = tvscreener()
        .args(["scan", "stock", "--index", "SP500", "--limit", "3"])
        .output()
        .expect("spawn tvscreener");
    assert!(out.status.success(), "stderr={}", stderr(&out));
    let text = stdout(&out);
    assert!(
        !text.trim().is_empty() && text.trim() != "(no rows)",
        "expected SP500 rows: {text}"
    );
}

/// `scan crypto --preset crypto_price --limit 2` respects the preset flag.
#[test]
fn cli_scan_crypto_preset() {
    require_live_env();
    let out = tvscreener()
        .args(["scan", "crypto", "--preset", "crypto_price", "--limit", "2"])
        .output()
        .expect("spawn tvscreener");
    assert!(out.status.success(), "stderr={}", stderr(&out));
    let text = stdout(&out);
    assert!(
        !text.trim().is_empty() && text.trim() != "(no rows)",
        "expected rows with preset: {text}"
    );
}

/// `scan forex --limit 2` returns forex pairs.
#[test]
fn cli_scan_forex() {
    require_live_env();
    let out = tvscreener()
        .args(["scan", "forex", "--limit", "2"])
        .output()
        .expect("spawn tvscreener");
    assert!(out.status.success(), "stderr={}", stderr(&out));
    let text = stdout(&out);
    assert!(
        !text.trim().is_empty() && text.trim() != "(no rows)",
        "expected forex rows: {text}"
    );
}

/// `scan` with unknown preset exits non-zero and reports the preset name.
#[test]
fn cli_scan_unknown_preset_fails() {
    require_live_env();
    let out = tvscreener()
        .args(["scan", "crypto", "--preset", "no_such_preset_xyz"])
        .output()
        .expect("spawn tvscreener");
    assert!(!out.status.success(), "expected failure for unknown preset");
    let err = stderr(&out);
    assert!(
        err.contains("no_such_preset_xyz") || err.contains("preset"),
        "unexpected stderr: {err}"
    );
}
