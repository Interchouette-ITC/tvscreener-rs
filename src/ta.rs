// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Technical-analysis helpers for computed recommendation columns.
//!
//! Mirrors Python `tvscreener.ta` (ADX, Awesome Oscillator, Bollinger Bands).

/// Buy / sell / neutral signal used by recommendation arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Bullish.
    Buy,
    /// Bearish.
    Sell,
    /// Neither.
    Neutral,
}

impl Signal {
    /// Arrow letter used by Python beautify (`↑ B` / `↓ S` / `- N`).
    #[must_use]
    pub const fn letter(self) -> &'static str {
        match self {
            Self::Buy => "↑ B",
            Self::Sell => "↓ S",
            Self::Neutral => "- N",
        }
    }
}

/// Returns true when `x` crossed above `y` between the previous and current bar.
#[must_use]
pub const fn crosses_up(x1: f64, x2: f64, y1: f64, y2: f64) -> bool {
    x1 > y1 && x2 < y2
}

/// ADX recommendation from DI crossover and ADX strength.
#[must_use]
pub const fn adx(
    adx_value: f64,
    dminus: f64,
    dplus: f64,
    dminus_old: f64,
    dplus_old: f64,
) -> Signal {
    if crosses_up(dplus, dplus_old, dminus, dminus_old) && adx_value > 20.0 {
        Signal::Buy
    } else if crosses_up(dminus, dminus_old, dplus, dplus_old) && adx_value > 20.0 {
        Signal::Sell
    } else {
        Signal::Neutral
    }
}

/// Awesome Oscillator recommendation (saucer / zero-line cross).
#[must_use]
pub const fn ao(ao_value: f64, ao_old_1: f64, ao_old_2: f64) -> Signal {
    if is_ao_bullish_saucer(ao_value, ao_old_1, ao_old_2)
        || is_ao_bullish_cross(ao_value, ao_old_1, ao_old_2)
    {
        Signal::Buy
    } else if is_ao_bearish_saucer(ao_value, ao_old_1, ao_old_2)
        || is_ao_bearish_cross(ao_value, ao_old_1, ao_old_2)
    {
        Signal::Sell
    } else {
        Signal::Neutral
    }
}

/// Bollinger lower-band signal: buy when close is below the band.
#[must_use]
pub const fn bb_lower(low_limit: f64, close: f64) -> Signal {
    if close < low_limit {
        Signal::Buy
    } else {
        Signal::Neutral
    }
}

/// Bollinger upper-band signal: sell when close is above the band.
#[must_use]
pub const fn bb_upper(up_limit: f64, close: f64) -> Signal {
    if close > up_limit {
        Signal::Sell
    } else {
        Signal::Neutral
    }
}

const fn is_ao_bearish_cross(ao_value: f64, ao_old_1: f64, ao_old_2: f64) -> bool {
    ao_value < 0.0 && ao_old_1 > 0.0 && ao_old_2 > 0.0
}

const fn is_ao_bullish_cross(ao_value: f64, ao_old_1: f64, ao_old_2: f64) -> bool {
    ao_value > 0.0 && ao_old_1 < 0.0 && ao_old_2 < 0.0
}

const fn is_ao_bullish_saucer(ao_value: f64, ao_old_1: f64, ao_old_2: f64) -> bool {
    ao_value > 0.0 && ao_value > ao_old_1 && ao_old_1 < ao_old_2
}

const fn is_ao_bearish_saucer(ao_value: f64, ao_old_1: f64, ao_old_2: f64) -> bool {
    ao_value < 0.0 && ao_value < ao_old_1 && ao_old_1 > ao_old_2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adx_buy_on_plus_di_cross_with_strength() {
        // +DI crosses above -DI, ADX > 20
        assert_eq!(adx(25.0, 10.0, 15.0, 12.0, 8.0), Signal::Buy);
    }

    #[test]
    fn adx_sell_on_minus_di_cross_with_strength() {
        assert_eq!(adx(25.0, 15.0, 10.0, 8.0, 12.0), Signal::Sell);
    }

    #[test]
    fn adx_neutral_when_weak() {
        assert_eq!(adx(10.0, 10.0, 15.0, 12.0, 8.0), Signal::Neutral);
    }

    #[test]
    fn ao_bullish_zero_cross() {
        assert_eq!(ao(0.5, -0.2, -0.4), Signal::Buy);
    }

    #[test]
    fn ao_bearish_zero_cross() {
        assert_eq!(ao(-0.5, 0.2, 0.4), Signal::Sell);
    }

    #[test]
    fn bb_signals() {
        assert_eq!(bb_lower(100.0, 99.0), Signal::Buy);
        assert_eq!(bb_lower(100.0, 101.0), Signal::Neutral);
        assert_eq!(bb_upper(100.0, 101.0), Signal::Sell);
        assert_eq!(bb_upper(100.0, 99.0), Signal::Neutral);
    }
}
