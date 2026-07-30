// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! TUI application model and scan configuration.

use crate::beautify::DEFAULT_TABLE_MAX_COLUMNS;
use crate::field::{default_fields, get_preset, Asset, FieldDef};
use crate::ScreenerRow;

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
}

impl AppModel {
    /// Builds a model after an initial scan (or empty rows on failure).
    #[must_use]
    pub fn new(config: ScanConfig, fields: Vec<FieldDef>, rows: Vec<ScreenerRow>) -> Self {
        let status = format!(
            "{}  rows={}  cols<={}",
            config.asset.as_str(),
            rows.len(),
            DEFAULT_TABLE_MAX_COLUMNS
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
        }
    }

    /// Records a successful scan.
    pub fn set_rows(&mut self, fields: Vec<FieldDef>, rows: Vec<ScreenerRow>) {
        self.fields = fields;
        self.rows = rows;
        self.error = None;
        self.scroll = 0;
        self.ticks = self.ticks.saturating_add(1);
        self.status = format!(
            "{}  rows={}  tick={}",
            self.config.asset.as_str(),
            self.rows.len(),
            self.ticks
        );
    }

    /// Records a scan failure (keeps previous rows).
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.ticks = self.ticks.saturating_add(1);
    }

    /// Toggles Results ↔ Help.
    pub const fn toggle_help(&mut self) {
        self.view = match self.view {
            ViewMode::Results => ViewMode::Help,
            ViewMode::Help => ViewMode::Results,
        };
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
