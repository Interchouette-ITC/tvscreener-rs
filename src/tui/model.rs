// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! TUI application model and scan configuration.

use std::time::{Duration, Instant};

use crate::beautify::DEFAULT_TABLE_MAX_COLUMNS;
use crate::field::{default_fields, get_preset, search_fields, Asset, FieldDef};
use crate::filter::{FieldCondition, FilterOperator};
use crate::ScreenerRow;

use super::scan::{preset_index, preset_options};

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
    /// Query builder (preset, limit, search, filters).
    Builder,
    /// Pretty-printed `build_payload()` JSON.
    Payload,
    /// Rust + CLI codegen snippets.
    Codegen,
    /// Key binding help.
    Help,
}

impl ViewMode {
    /// Cycles to the next primary pane (skips Help).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Results => Self::Builder,
            Self::Builder => Self::Payload,
            Self::Payload => Self::Codegen,
            Self::Codegen | Self::Help => Self::Results,
        }
    }

    /// Short label for the title bar.
    #[must_use]
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Results => "1 Results",
            Self::Builder => "2 Builder",
            Self::Payload => "3 Payload",
            Self::Codegen => "4 Codegen",
            Self::Help => "Help",
        }
    }
}

/// Builder row focus for `j` / `k` navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuilderFocus {
    /// Asset class picker.
    Asset,
    /// Field preset picker.
    #[default]
    Preset,
    /// Row limit.
    Limit,
    /// Name search text.
    Search,
    /// Sort field and direction.
    Sort,
    /// Stock markets CSV.
    Markets,
    /// Stock index CSV.
    Index,
    /// Filter list.
    Filters,
}

/// Assets shown in the builder picker (fixed order).
pub const BUILDER_ASSETS: [Asset; 6] = [
    Asset::Stock,
    Asset::Crypto,
    Asset::Forex,
    Asset::Bond,
    Asset::Futures,
    Asset::Coin,
];

/// Text input sub-mode inside the builder pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// No active input.
    #[default]
    None,
    /// Editing search text.
    Search,
    /// Editing sort field text.
    Sort,
    /// Editing stock markets CSV.
    Markets,
    /// Editing stock index CSV.
    Index,
    /// Picking a field from catalog search hits.
    FilterField,
    /// Cycling filter operator before value entry.
    FilterOp,
    /// Typing filter right-hand value.
    FilterValue,
}

/// Operators offered in the simple filter wizard.
pub const FILTER_OPS: [FilterOperator; 6] = [
    FilterOperator::Above,
    FilterOperator::Below,
    FilterOperator::AboveOrEqual,
    FilterOperator::BelowOrEqual,
    FilterOperator::Equal,
    FilterOperator::Match,
];

/// Builder pane UI state (not sent to the API until refresh).
#[derive(Debug, Clone)]
pub struct BuilderUi {
    /// Sorted preset names for the current asset (includes `(defaults)`).
    pub preset_options: Vec<String>,
    /// Index into [`Self::preset_options`].
    pub preset_index: usize,
    /// Highlighted builder row.
    pub focus: BuilderFocus,
    /// Selected filter row (`d` removes this one).
    pub filter_selected: usize,
    /// Active text input, if any.
    pub input_mode: InputMode,
    /// Buffer for search / field query / filter value.
    pub input_buf: String,
    /// Field catalog hits while adding a filter.
    pub field_matches: Vec<FieldDef>,
    /// Selected index in [`Self::field_matches`].
    pub field_pick: usize,
    /// Index into [`FILTER_OPS`] while adding a filter.
    pub op_index: usize,
    /// Draft field name between field pick and value entry.
    pub draft_field: Option<String>,
}

impl BuilderUi {
    /// Builds picker state from scan config.
    #[must_use]
    pub fn from_config(config: &ScanConfig) -> Self {
        let preset_options = preset_options(config.asset);
        let preset_index = preset_index(config.asset, config.preset.as_deref());
        Self {
            preset_options,
            preset_index,
            focus: BuilderFocus::default(),
            filter_selected: 0,
            input_mode: InputMode::default(),
            input_buf: String::new(),
            field_matches: Vec::new(),
            field_pick: 0,
            op_index: 0,
            draft_field: None,
        }
    }

    /// Refreshes preset list after asset changes (asset is fixed at launch).
    pub fn sync_preset_index(&mut self, config: &ScanConfig) {
        self.preset_options = preset_options(config.asset);
        self.preset_index = preset_index(config.asset, config.preset.as_deref());
    }

    /// Updates field matches from the current query buffer.
    pub fn refresh_field_matches(&mut self, asset: Asset) {
        self.field_matches = search_fields(asset, &self.input_buf);
        if self.field_pick >= self.field_matches.len() && !self.field_matches.is_empty() {
            self.field_pick = self.field_matches.len() - 1;
        }
    }

    /// Clears filter-add wizard state.
    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::None;
        self.input_buf.clear();
        self.field_matches.clear();
        self.field_pick = 0;
        self.op_index = 0;
        self.draft_field = None;
    }
}

/// CLI / env scan parameters for the TUI.
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
    /// Sort field (const, technical, or label).
    pub sort_by: Option<String>,
    /// Sort ascending when true (default descending).
    pub ascending: bool,
    /// Field conditions applied via `where_condition`.
    pub filters: Vec<FieldCondition>,
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
    /// Builder pane state.
    pub builder: BuilderUi,
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
    /// Vertical scroll offset (results rows or text panes).
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
        let builder = BuilderUi::from_config(&config);
        let status = format!(
            "{}  rows={}  {}",
            config.asset.as_str(),
            rows.len(),
            refresh_mode_label(watch, watch_interval),
        );
        Self {
            builder,
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
            ViewMode::Help => ViewMode::Results,
            _ => ViewMode::Help,
        };
        self.scroll = 0;
    }

    /// Cycles Results → Builder → Payload → Codegen.
    pub fn cycle_view(&mut self) {
        if self.view == ViewMode::Help {
            self.view = ViewMode::Results;
        } else {
            self.view = self.view.next();
        }
        self.scroll = 0;
    }

    /// Jumps to a numbered pane (`1`–`4`).
    pub const fn set_view_number(&mut self, n: u8) {
        self.view = match n {
            1 => ViewMode::Results,
            2 => ViewMode::Builder,
            3 => ViewMode::Payload,
            4 => ViewMode::Codegen,
            _ => return,
        };
        self.scroll = 0;
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

    /// Scrolls the active pane by `delta` lines.
    pub fn scroll_by(&mut self, delta: isize) {
        if delta >= 0 {
            self.scroll = self
                .scroll
                .saturating_add(usize::try_from(delta).unwrap_or(0));
        } else {
            let step = usize::try_from(delta.wrapping_neg()).unwrap_or(0);
            self.scroll = self.scroll.saturating_sub(step);
        }
    }

    /// Moves builder focus up/down.
    pub fn builder_focus_by(&mut self, delta: isize) {
        let rows = [
            BuilderFocus::Asset,
            BuilderFocus::Preset,
            BuilderFocus::Limit,
            BuilderFocus::Search,
            BuilderFocus::Sort,
            BuilderFocus::Markets,
            BuilderFocus::Index,
            BuilderFocus::Filters,
        ];
        let idx = rows
            .iter()
            .position(|&f| f == self.builder.focus)
            .unwrap_or(0);
        let next = if delta >= 0 {
            (idx + 1).min(rows.len() - 1)
        } else {
            idx.saturating_sub(1)
        };
        self.builder.focus = rows[next];
    }

    /// Cycles asset when the Asset row is focused.
    pub fn step_asset(&mut self, delta: isize) {
        let idx = BUILDER_ASSETS
            .iter()
            .position(|&a| a == self.config.asset)
            .unwrap_or(0);
        let len = BUILDER_ASSETS.len();
        let next = if delta >= 0 {
            (idx + 1) % len
        } else {
            (idx + len - 1) % len
        };
        if BUILDER_ASSETS[next] != self.config.asset {
            self.config.asset = BUILDER_ASSETS[next];
            self.on_asset_changed();
        }
    }

    /// Resets config after an asset switch in the builder.
    pub fn on_asset_changed(&mut self) {
        self.config.preset = None;
        self.config.filters.clear();
        if self.config.asset != Asset::Stock {
            self.config.markets = None;
            self.config.index = None;
        }
        self.builder.sync_preset_index(&self.config);
        self.builder.filter_selected = 0;
        self.builder.cancel_input();
        self.rows.clear();
        self.fields = default_fields(self.config.asset);
        self.status = format!(
            "{}  asset changed — Enter to scan",
            self.config.asset.as_str()
        );
        self.error = None;
        self.scroll = 0;
    }

    /// Toggles sort direction when the Sort row is focused.
    pub const fn toggle_sort_ascending(&mut self) {
        self.config.ascending = !self.config.ascending;
    }

    /// Steps the preset picker and updates config.
    pub fn step_preset(&mut self, delta: isize) {
        let len = self.builder.preset_options.len();
        if len == 0 {
            return;
        }
        let idx = self.builder.preset_index;
        let next = if delta >= 0 {
            (idx + 1).min(len - 1)
        } else {
            idx.saturating_sub(1)
        };
        self.builder.preset_index = next;
        super::scan::apply_preset_index(&mut self.config, next);
        self.builder.sync_preset_index(&self.config);
    }

    /// Adjusts row limit (minimum 1, maximum 500).
    pub fn step_limit(&mut self, delta: i32) {
        let next = (i64::from(self.config.limit) + i64::from(delta)).clamp(1, 500);
        self.config.limit = u32::try_from(next).unwrap_or(1);
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

    fn stock_model() -> AppModel {
        AppModel::new(
            ScanConfig {
                asset: Asset::Stock,
                preset: None,
                from: 0,
                limit: 5,
                search: None,
                markets: Some("AMERICA".into()),
                index: Some("SP500".into()),
                sort_by: None,
                ascending: false,
                filters: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
            false,
            DEFAULT_WATCH_INTERVAL_SECS,
        )
    }

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
                sort_by: None,
                ascending: false,
                filters: Vec::new(),
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

    #[test]
    fn cycle_view_order() {
        let mut model = empty_model(false, DEFAULT_WATCH_INTERVAL_SECS);
        assert_eq!(model.view, ViewMode::Results);
        model.cycle_view();
        assert_eq!(model.view, ViewMode::Builder);
        model.cycle_view();
        assert_eq!(model.view, ViewMode::Payload);
        model.cycle_view();
        assert_eq!(model.view, ViewMode::Codegen);
    }

    #[test]
    fn step_limit_clamps() {
        let mut model = empty_model(false, DEFAULT_WATCH_INTERVAL_SECS);
        model.step_limit(1000);
        assert_eq!(model.config.limit, 500);
        model.step_limit(-1000);
        assert_eq!(model.config.limit, 1);
    }

    #[test]
    fn step_asset_clears_markets_when_leaving_stock() {
        let mut model = stock_model();
        model.step_asset(1);
        assert_eq!(model.config.asset, Asset::Crypto);
        assert!(model.config.markets.is_none());
        assert!(model.config.index.is_none());
        assert!(model.config.filters.is_empty());
        assert!(model.config.preset.is_none());
        assert!(model.status.contains("asset changed"));
    }

    #[test]
    fn builder_focus_includes_asset_and_sort() {
        let mut model = empty_model(false, DEFAULT_WATCH_INTERVAL_SECS);
        model.builder.focus = BuilderFocus::Asset;
        model.builder_focus_by(1);
        assert_eq!(model.builder.focus, BuilderFocus::Preset);
        model.builder.focus = BuilderFocus::Search;
        model.builder_focus_by(1);
        assert_eq!(model.builder.focus, BuilderFocus::Sort);
        model.builder_focus_by(1);
        assert_eq!(model.builder.focus, BuilderFocus::Markets);
    }

    #[test]
    fn toggle_sort_ascending_flips_flag() {
        let mut model = empty_model(false, DEFAULT_WATCH_INTERVAL_SECS);
        assert!(!model.config.ascending);
        model.toggle_sort_ascending();
        assert!(model.config.ascending);
    }
}
