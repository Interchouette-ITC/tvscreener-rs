// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Basic crate-link checks.

use tvscreener::core::Screener;
use tvscreener::field::FieldDef;
use tvscreener::util::{get_url, request_headers, SCANNER_ORIGIN, TRADINGVIEW_ORIGIN};

#[test]
fn get_url_uses_scanner_origin() {
    assert!(get_url("crypto").starts_with(SCANNER_ORIGIN));
}

#[test]
fn screener_builder_and_headers() {
    let mut s = Screener::new("crypto");
    s.select([FieldDef::new("Name", "name")]).set_range(0, 10);
    assert_eq!(s.selected_fields().len(), 1);
    assert_eq!(s.range(), (0, 10));
    assert_eq!(
        request_headers()
            .get(reqwest::header::ORIGIN)
            .unwrap()
            .to_str()
            .unwrap(),
        TRADINGVIEW_ORIGIN
    );
}
