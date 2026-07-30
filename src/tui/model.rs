// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! TUI application model and scan configuration.

use std::time::{Duration, Instant};

use crate::beautify::DEFAULT_TABLE_MAX_COLUMNS;
use crate::field::{default_fields, get_preset, Asset, FieldDef};
use crate::ScreenerRow;

/// Default auto-refresh interval when watch mode is on (seconds).
pub const DEFAULT_WATCH_INTERVAL_SECS: f64 = 30.0;

/// Minimum gap between TUI scans (manual `r` or watch).
///
/// Stricter than library [`crate::util::MIN_STREAM_INTERVAL_SECS`] (1s), which is only an
/// API floor for `stream()`, not a UI default.
pub const MIN_TUI_REFRESH_SECS: f64 = 10.0;

/// Which pane the TUI is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Beautified results table.
    #[default]
    Results,
    /// Key binding help.
    Help,
}

/// CLI / env scan parameters for the scaffold TUI.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Screener asset class.
    pub asset: Asset,
    /// Optional field preset name.
    pub preset: Option<String>,
    /// Result window start.
    pub from: u32,
    /// Max rows.
    pub limit: u32,
    /// Optional name search.
    pub search: Option<String>,
    /// Stock markets CSV (const or wire).
    pub markets: Option<String>,
    /// Stock index CSV (const or wire).
    pub index: Option<String>,
}

impl ScanConfig {
    /// Resolves selected fields (preset or asset defaults).
    ///
    /// # Errors
    ///
    /// Returns catalog errors from [`get_preset`].
    pub fn resolve_fields(&self) -> crate::Result<Vec<FieldDef>> {
        if let Some(name) = self.preset.as_deref() {
            return get_preset(name);
        }
        Ok(default_fields(self.asset))
    }
}

/// Pane model: config, last rows, status line, active view.
#[derive(Debug, Clone)]
pub struct AppModel {
    /// Scan parameters.
    pub config: ScanConfig,
    /// Fields used for the last scan / table columns.
    pub fields: Vec<FieldDef>,
    /// Last scan rows.
    pub rows: Vec<ScreenerRow>,
    /// Status / branding line (non-error).
    pub status: String,
    /// Last error message, if any.
    pub error: Option<String>,
    /// Active view.
    pub view: ViewMode,
    /// Vertical scroll offset into rows.
    pub scroll: usize,
    /// Refresh counter.
    pub ticks: u64,
    /// Max data columns in the results table.
    pub max_columns: usize,
    /// When true, auto-refresh on [`Self::watch_interval`].
    pub watch: bool,
    /// Auto-refresh period (already floored to [`MIN_TUI_REFRESH_SECS`]).
    pub watch_interval: Duration,
    /// Instant of the last completed scan attempt (success or failure).
    pub last_scan_at: Instant,
}

impl AppModel {
    /// Builds a model after an initial scan (or empty rows on failure).
    #[must_use]
    pub fn new(
        config: ScanConfig,
        fields: Vec<FieldDef>,
        rows: Vec<ScreenerRow>,
        watch: bool,
        watch_interval_secs: f64,
    ) -> Self {
        let watch_interval = Duration::from_secs_f64(watch_interval_secs.max(MIN_TUI_REFRESH_SECS));
        let status = format!(
            "{}  rows={}  {}",
            config.asset.as_str(),
            rows.len(),
            refresh_mode_label(watch, watch_interval),
        );
        Self {
            config,
            fields,
            rows,
            status,
            error: None,
            view: ViewMode::Results,
            scroll: 0,
            ticks: 0,
            max_columns: DEFAULT_TABLE_MAX_COLUMNS,
            watch,
            watch_interval,
            last_scan_at: Instant::now(),
        }
    }

    /// Records a successful scan.
    pub fn set_rows(&mut self, fields: Vec<FieldDef>, rows: Vec<ScreenerRow>) {
        self.fields = fields;
        self.rows = rows;
        self.error = None;
        self.scroll = 0;
        self.ticks = self.ticks.saturating_add(1);
        self.last_scan_at = Instant::now();
        self.status = format!(
            "{}  rows={}  tick={}  {}",
            self.config.asset.as_str(),
            self.rows.len(),
            self.ticks,
            refresh_mode_label(self.watch, self.watch_interval),
        );
    }

    /// Records a scan failure (keeps previous rows).
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.ticks = self.ticks.saturating_add(1);
        self.last_scan_at = Instant::now();
    }

    /// Toggles Results ↔ Help.
    pub const fn toggle_help(&mut self) {
        self.view = match self.view {
            ViewMode::Results => ViewMode::Help,
            ViewMode::Help => ViewMode::Results,
        };
    }

    /// Toggles opt-in auto-refresh (off by default).
    pub fn toggle_watch(&mut self) {
        self.watch = !self.watch;
        self.status = format!(
            "{}  rows={}  {}",
            self.config.asset.as_str(),
            self.rows.len(),
            refresh_mode_label(self.watch, self.watch_interval),
        );
    }

    /// Seconds remaining before another scan is allowed (`0` when ready).
    #[must_use]
    pub fn cooldown_secs_remaining(&self) -> u64 {
        let min_gap = Duration::from_secs_f64(MIN_TUI_REFRESH_SECS);
        let elapsed = self.last_scan_at.elapsed();
        min_gap.saturating_sub(elapsed).as_secs()
    }

    /// Returns true when a manual or watch refresh may run (honors TUI floor).
    #[must_use]
    pub fn can_refresh(&self) -> bool {
        self.cooldown_secs_remaining() == 0
    }

    /// Returns true when watch mode is on and the watch interval has elapsed.
    #[must_use]
    pub fn watch_due(&self) -> bool {
        self.watch && self.last_scan_at.elapsed() >= self.watch_interval
    }

    /// Scrolls the results table by `delta` rows.
    pub fn scroll_by(&mut self, delta: isize) {
        let len = self.rows.len();
        if len == 0 {
            self.scroll = 0;
            return;
        }
        let max = len - 1;
        if delta >= 0 {
            self.scroll = self
                .scroll
                .saturating_add(usize::try_from(delta).unwrap_or(0));
            if self.scroll > max {
                self.scroll = max;
            }
        } else {
            let step = usize::try_from(delta.wrapping_neg()).unwrap_or(0);
            self.scroll = self.scroll.saturating_sub(step);
        }
    }
}

fn refresh_mode_label(watch: bool, interval: Duration) -> String {
    if watch {
        format!("watch {}s", interval.as_secs().max(1))
    } else {
        "manual".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_model(watch: bool, interval: f64) -> AppModel {
        AppModel::new(
            ScanConfig {
                asset: Asset::Crypto,
                preset: None,
                from: 0,
                limit: 5,
                search: None,
                markets: None,
                index: None,
            },
            Vec::new(),
            Vec::new(),
            watch,
            interval,
        )
    }

    #[test]
    fn watch_off_by_default_and_interval_floored() {
        let model = empty_model(false, 1.0);
        assert!(!model.watch);
        assert!(!model.watch_due());
        assert_eq!(
            model.watch_interval,
            Duration::from_secs_f64(MIN_TUI_REFRESH_SECS)
        );
        assert!(!model.can_refresh());
    }

    #[test]
    fn toggle_watch_updates_label() {
        let mut model = empty_model(false, DEFAULT_WATCH_INTERVAL_SECS);
        model.toggle_watch();
        assert!(model.watch);
        assert!(model.status.contains("watch"));
        model.toggle_watch();
        assert!(!model.watch);
        assert!(model.status.contains("manual"));
    }
}
