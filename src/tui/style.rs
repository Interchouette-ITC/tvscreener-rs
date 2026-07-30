// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Map [`CellTone`] to Ratatui colors (TradingView-ish RGB from Python beauty).

use ratatui::style::Color;

use crate::beautify::CellTone;

/// Ratatui foreground color for a beautify tone.
#[must_use]
pub const fn tone_color(tone: CellTone) -> Color {
    let (r, g, b) = tone.rgb();
    Color::Rgb(r, g, b)
}
