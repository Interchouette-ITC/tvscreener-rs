// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Checks for the default `tvscreener` CLI (no scanner HTTP).

use std::process::Command;

fn tvscreener() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tvscreener"))
}

fn stdout_utf8(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn cli_help_lists_scan_and_payload() {
    let out = tvscreener()
        .arg("--help")
        .output()
        .expect("run tvscreener --help");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_utf8(&out);
    assert!(text.contains("scan"), "{text}");
    assert!(text.contains("payload"), "{text}");
    assert!(text.contains("presets"), "{text}");
}

#[test]
fn cli_payload_crypto_prints_json_columns() {
    let out = tvscreener()
        .args(["payload", "crypto", "--limit", "2"])
        .output()
        .expect("run tvscreener payload");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_utf8(&out);
    let v: serde_json::Value = serde_json::from_str(&text).expect("payload JSON");
    let cols = v
        .get("columns")
        .and_then(|c| c.as_array())
        .expect("columns array");
    assert!(!cols.is_empty(), "expected non-empty columns");
}

#[test]
fn cli_presets_lists_crypto_price() {
    let out = tvscreener()
        .arg("presets")
        .output()
        .expect("run tvscreener presets");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_utf8(&out);
    assert!(
        text.lines().any(|l| l.trim() == "crypto_price"),
        "missing crypto_price in:\n{text}"
    );
}

#[test]
fn cli_markets_lists_america() {
    let out = tvscreener()
        .arg("markets")
        .output()
        .expect("run tvscreener markets");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_utf8(&out);
    assert!(text.contains("AMERICA"), "{text}");
}

#[test]
fn cli_sectors_lists_technology_services() {
    let out = tvscreener()
        .arg("sectors")
        .output()
        .expect("run tvscreener sectors");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_utf8(&out);
    assert!(
        text.contains("TECHNOLOGY_SERVICES") && text.contains("Technology Services"),
        "{text}"
    );
}

#[test]
fn cli_fields_search_finds_volume() {
    let out = tvscreener()
        .args(["fields", "volume", "--asset", "stock", "--limit", "5"])
        .output()
        .expect("run tvscreener fields");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_utf8(&out);
    assert!(
        text.to_ascii_lowercase().contains("volume"),
        "expected volume hit in:\n{text}"
    );
}

#[test]
fn cli_payload_rejects_markets_on_crypto() {
    let out = tvscreener()
        .args(["payload", "crypto", "--markets", "AMERICA"])
        .output()
        .expect("run tvscreener payload crypto --markets");
    assert!(
        !out.status.success(),
        "expected failure when --markets used on crypto"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("markets") || err.contains("stock"),
        "unexpected stderr:\n{err}"
    );
}

#[test]
fn cli_payload_stock_index_sp500_symbolset() {
    let out = tvscreener()
        .args(["payload", "stock", "--index", "SP500", "--limit", "2"])
        .output()
        .expect("run tvscreener payload stock --index");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_str(&stdout_utf8(&out)).expect("payload JSON");
    let symbolset = v
        .pointer("/symbols/symbolset")
        .and_then(|s| s.as_array())
        .expect("symbols.symbolset");
    assert!(
        symbolset.iter().any(|s| s.as_str() == Some("SYML:SP;SPX")),
        "expected SYML:SP;SPX in {symbolset:?}"
    );
}

#[test]
fn cli_payload_stock_markets_america() {
    let out = tvscreener()
        .args(["payload", "stock", "--markets", "AMERICA", "--limit", "2"])
        .output()
        .expect("run tvscreener payload stock --markets");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_str(&stdout_utf8(&out)).expect("payload JSON");
    assert_eq!(v["markets"], serde_json::json!(["america"]));
}
