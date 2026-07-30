// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener-tui` - Ratatui screener TUI.
//!
//! ```bash
//! cargo run --bin tvscreener-tui -- crypto --limit 10
//! make run-tui ARGS='crypto --preset crypto_price --limit 5'
//! make run-tui ARGS='crypto --watch --interval 30'
//! ```

use std::io::{self, stdout};
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tvscreener::filter::FieldCondition;
use tvscreener::query_config::{parse_filter_conditions_from_cli, parse_filter_value};
use tvscreener::tui::{
    configure_screener, copy_via_osc52, draw, hard_reset_tty, inside_gnu_screen,
    install_panic_hook, install_signal_handlers, is_quit_key, payload_pretty, render_codegen,
    AppModel, BuilderFocus, InputMode, ScanConfig, TerminalGuard, ViewMode,
    DEFAULT_WATCH_INTERVAL_SECS, FILTER_OPS, MIN_TUI_REFRESH_SECS, STOP,
};
use tvscreener::{field::Asset, ScreenerRow};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AssetArg {
    Stock,
    Crypto,
    Forex,
    Bond,
    Futures,
    Coin,
}

impl AssetArg {
    const fn asset(self) -> Asset {
        match self {
            Self::Stock => Asset::Stock,
            Self::Crypto => Asset::Crypto,
            Self::Forex => Asset::Forex,
            Self::Bond => Asset::Bond,
            Self::Futures => Asset::Futures,
            Self::Coin => Asset::Coin,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "tvscreener-tui",
    about = "TradingView screener Ratatui TUI",
    version
)]
struct Cli {
    /// Screener asset class.
    asset: AssetArg,
    /// Preset name (see `tvscreener presets`).
    #[arg(long)]
    preset: Option<String>,
    /// Result window start (inclusive).
    #[arg(long, default_value_t = 0)]
    from: u32,
    /// Max rows.
    #[arg(long, default_value_t = 25)]
    limit: u32,
    /// Name/description search text.
    #[arg(long)]
    search: Option<String>,
    /// Stock markets CSV (const or wire), e.g. `AMERICA`.
    #[arg(long)]
    markets: Option<String>,
    /// Stock index CSV (const or wire), e.g. `SP500`.
    #[arg(long)]
    index: Option<String>,
    /// Repeatable `FIELD:OP:VALUE` filter, e.g. `close:greater:100`.
    #[arg(long = "filter", action = ArgAction::Append)]
    filter: Vec<String>,
    /// JSON array or object of `{field, op, value}` filters.
    #[arg(long)]
    filters: Option<String>,
    /// Sort field (const, technical, or label).
    #[arg(long)]
    sort_by: Option<String>,
    /// Sort ascending (default: descending).
    #[arg(long, default_value_t = false)]
    ascending: bool,
    /// Start with auto-refresh enabled (still floored to 10s).
    #[arg(long)]
    watch: bool,
    /// Watch interval in seconds (default 30; minimum 10).
    #[arg(long, default_value_t = DEFAULT_WATCH_INTERVAL_SECS)]
    interval: f64,
    /// Force screener debug logs (also: `TVSCREENER_DEBUG=1`).
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tvscreener::logging::init_logging();

    let filters =
        parse_filter_conditions_from_cli(&cli.filter, cli.filters.as_deref()).context("filters")?;

    let config = ScanConfig {
        asset: cli.asset.asset(),
        preset: cli.preset.clone(),
        from: cli.from,
        limit: cli.limit,
        search: cli.search.clone(),
        markets: cli.markets.clone(),
        index: cli.index.clone(),
        sort_by: cli.sort_by.clone(),
        ascending: cli.ascending,
        filters,
    };

    let interval = cli.interval.max(MIN_TUI_REFRESH_SECS);

    let fields = config.resolve_fields().context("resolve fields")?;
    let (fields, rows) = match run_scan(&config, cli.debug).await {
        Ok(rows) => (fields, rows),
        Err(err) => {
            eprintln!("initial scan failed: {err:#}");
            (fields, Vec::new())
        }
    };

    let mut model = AppModel::new(config, fields, rows, cli.watch, interval);
    if model.rows.is_empty() && model.error.is_none() {
        model.status = format!("{}  (no rows)", model.status);
    }

    install_signal_handlers();
    install_panic_hook();
    let use_alt = !inside_gnu_screen();
    let _guard = TerminalGuard::enter(use_alt)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_loop(&mut terminal, &mut model, cli.debug).await;

    let _ = terminal.clear();
    hard_reset_tty();
    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    model: &mut AppModel,
    debug: bool,
) -> Result<()> {
    loop {
        if STOP.load(Ordering::SeqCst) {
            break;
        }

        if model.watch_due() {
            refresh(model, debug).await;
        }

        terminal.draw(|frame| draw(frame, model))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if is_quit_key(key.code, key.modifiers) {
            break;
        }

        if model.builder.input_mode != InputMode::None && handle_input_key(model, key.code) {
            continue;
        }

        handle_global_key(model, key.code, debug).await?;
    }
    Ok(())
}

async fn handle_global_key(model: &mut AppModel, code: KeyCode, debug: bool) -> Result<()> {
    match code {
        KeyCode::Tab => model.cycle_view(),
        KeyCode::Char('h') => model.toggle_help(),
        KeyCode::Char('a') => model.toggle_watch(),
        KeyCode::Char('c') => copy_active_pane(model),
        KeyCode::Char('r') => {
            if model.can_refresh() {
                refresh(model, debug).await;
            } else {
                let wait = model.cooldown_secs_remaining();
                model.status = format!("cooldown {wait}s (min {MIN_TUI_REFRESH_SECS}s)");
            }
        }
        KeyCode::Char('1') => model.set_view_number(1),
        KeyCode::Char('2') => model.set_view_number(2),
        KeyCode::Char('3') => model.set_view_number(3),
        KeyCode::Char('4') => model.set_view_number(4),
        KeyCode::Up => model.scroll_by(-1),
        KeyCode::Down => model.scroll_by(1),
        KeyCode::PageUp => model.scroll_by(-10),
        KeyCode::PageDown => model.scroll_by(10),
        KeyCode::Enter if model.view == ViewMode::Builder && model.can_refresh() => {
            refresh(model, debug).await;
        }
        _ if model.view == ViewMode::Builder => handle_builder_key(model, code),
        _ => {}
    }
    Ok(())
}

fn handle_builder_key(model: &mut AppModel, code: KeyCode) {
    match code {
        KeyCode::Left if model.builder.input_mode == InputMode::None => match model.builder.focus {
            BuilderFocus::Asset => model.step_asset(-1),
            BuilderFocus::Preset => model.step_preset(-1),
            _ => {}
        },
        KeyCode::Right if model.builder.input_mode == InputMode::None => {
            match model.builder.focus {
                BuilderFocus::Asset => model.step_asset(1),
                BuilderFocus::Preset => model.step_preset(1),
                _ => {}
            }
        }
        KeyCode::Char('+' | '=') => model.step_limit(1),
        KeyCode::Char('-') => model.step_limit(-1),
        KeyCode::Char('s') => start_search_input(model),
        KeyCode::Char('t') if model.builder.focus == BuilderFocus::Sort => {
            start_sort_input(model);
        }
        KeyCode::Char('u')
            if model.builder.focus == BuilderFocus::Sort
                && model.builder.input_mode == InputMode::None =>
        {
            model.toggle_sort_ascending();
        }
        KeyCode::Char('m')
            if model.builder.focus == BuilderFocus::Markets
                && model.config.asset == Asset::Stock =>
        {
            start_markets_input(model);
        }
        KeyCode::Char('i')
            if model.builder.focus == BuilderFocus::Index && model.config.asset == Asset::Stock =>
        {
            start_index_input(model);
        }
        KeyCode::Char('f') => start_filter_input(model),
        KeyCode::Char('d') => remove_selected_filter(model),
        KeyCode::Char('o') if model.builder.input_mode == InputMode::FilterOp => {
            cycle_filter_op(model, 1);
        }
        KeyCode::Char('j') => handle_builder_j(model),
        KeyCode::Char('k') => handle_builder_k(model),
        _ => {}
    }
}

fn handle_builder_j(model: &mut AppModel) {
    match model.builder.input_mode {
        InputMode::FilterField => step_field_pick(model, 1),
        InputMode::FilterOp => cycle_filter_op(model, 1),
        InputMode::None if model.builder.focus == BuilderFocus::Filters => {
            step_filter_select(model, 1);
        }
        InputMode::None => model.builder_focus_by(1),
        InputMode::Search
        | InputMode::Sort
        | InputMode::Markets
        | InputMode::Index
        | InputMode::FilterValue => {}
    }
}

fn handle_builder_k(model: &mut AppModel) {
    match model.builder.input_mode {
        InputMode::FilterField => step_field_pick(model, -1),
        InputMode::FilterOp => cycle_filter_op(model, -1),
        InputMode::None if model.builder.focus == BuilderFocus::Filters => {
            step_filter_select(model, -1);
        }
        InputMode::None => model.builder_focus_by(-1),
        InputMode::Search
        | InputMode::Sort
        | InputMode::Markets
        | InputMode::Index
        | InputMode::FilterValue => {}
    }
}

/// Returns true when the key was consumed by input mode handling.
fn handle_input_key(model: &mut AppModel, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc => {
            model.builder.cancel_input();
            true
        }
        KeyCode::Backspace => {
            model.builder.input_buf.pop();
            if model.builder.input_mode == InputMode::FilterField {
                model.builder.refresh_field_matches(model.config.asset);
            }
            true
        }
        KeyCode::Enter => {
            commit_input(model);
            true
        }
        KeyCode::Char(c) if !c.is_control() => {
            if model.builder.input_mode == InputMode::FilterOp {
                if c == 'o' {
                    cycle_filter_op(model, 1);
                }
            } else {
                model.builder.input_buf.push(c);
                if model.builder.input_mode == InputMode::FilterField {
                    model.builder.refresh_field_matches(model.config.asset);
                }
            }
            true
        }
        _ => false,
    }
}

fn start_search_input(model: &mut AppModel) {
    model.builder.cancel_input();
    model.builder.input_mode = InputMode::Search;
    model.builder.input_buf = model.config.search.clone().unwrap_or_default();
}

fn start_sort_input(model: &mut AppModel) {
    model.builder.cancel_input();
    model.builder.input_mode = InputMode::Sort;
    model.builder.input_buf = model.config.sort_by.clone().unwrap_or_default();
}

fn start_markets_input(model: &mut AppModel) {
    model.builder.cancel_input();
    model.builder.input_mode = InputMode::Markets;
    model.builder.input_buf = model.config.markets.clone().unwrap_or_default();
}

fn start_index_input(model: &mut AppModel) {
    model.builder.cancel_input();
    model.builder.input_mode = InputMode::Index;
    model.builder.input_buf = model.config.index.clone().unwrap_or_default();
}

fn start_filter_input(model: &mut AppModel) {
    model.builder.cancel_input();
    model.builder.input_mode = InputMode::FilterField;
    model.builder.refresh_field_matches(model.config.asset);
}

fn step_field_pick(model: &mut AppModel, delta: isize) {
    let len = model.builder.field_matches.len();
    if len == 0 {
        return;
    }
    let idx = model.builder.field_pick;
    model.builder.field_pick = if delta >= 0 {
        (idx + 1).min(len - 1)
    } else {
        idx.saturating_sub(1)
    };
}

fn step_filter_select(model: &mut AppModel, delta: isize) {
    let len = model.config.filters.len();
    if len == 0 {
        return;
    }
    let idx = model.builder.filter_selected;
    model.builder.filter_selected = if delta >= 0 {
        (idx + 1).min(len - 1)
    } else {
        idx.saturating_sub(1)
    };
}

const fn cycle_filter_op(model: &mut AppModel, delta: isize) {
    let len = FILTER_OPS.len();
    let idx = model.builder.op_index;
    model.builder.op_index = if delta >= 0 {
        (idx + 1) % len
    } else {
        (idx + len - 1) % len
    };
}

fn commit_input(model: &mut AppModel) {
    match model.builder.input_mode {
        InputMode::Search => {
            let text = model.builder.input_buf.trim().to_string();
            model.config.search = if text.is_empty() { None } else { Some(text) };
            model.builder.cancel_input();
        }
        InputMode::Sort => {
            let text = model.builder.input_buf.trim().to_string();
            model.config.sort_by = if text.is_empty() { None } else { Some(text) };
            model.builder.cancel_input();
        }
        InputMode::Markets => {
            let text = model.builder.input_buf.trim().to_string();
            model.config.markets = if text.is_empty() { None } else { Some(text) };
            model.builder.cancel_input();
        }
        InputMode::Index => {
            let text = model.builder.input_buf.trim().to_string();
            model.config.index = if text.is_empty() { None } else { Some(text) };
            model.builder.cancel_input();
        }
        InputMode::FilterField => {
            let Some(field) = model
                .builder
                .field_matches
                .get(model.builder.field_pick)
                .map(|f| f.field_name.clone())
            else {
                model.status = "pick a field (type to search, j/k, Enter)".into();
                return;
            };
            model.builder.draft_field = Some(field);
            model.builder.input_buf.clear();
            model.builder.input_mode = InputMode::FilterOp;
        }
        InputMode::FilterOp => {
            model.builder.input_mode = InputMode::FilterValue;
            model.builder.input_buf.clear();
        }
        InputMode::FilterValue => {
            let Some(left) = model.builder.draft_field.clone() else {
                model.builder.cancel_input();
                return;
            };
            let op = FILTER_OPS
                .get(model.builder.op_index)
                .copied()
                .unwrap_or(FILTER_OPS[0]);
            let value = parse_filter_value(&model.builder.input_buf);
            model
                .config
                .filters
                .push(FieldCondition::new(left, op, value));
            model.builder.filter_selected = model.config.filters.len().saturating_sub(1);
            model.builder.cancel_input();
            model.status = format!("filter added ({} total)", model.config.filters.len());
        }
        InputMode::None => {}
    }
}

fn remove_selected_filter(model: &mut AppModel) {
    let idx = model.builder.filter_selected;
    if idx < model.config.filters.len() {
        model.config.filters.remove(idx);
        if model.builder.filter_selected > 0
            && model.builder.filter_selected >= model.config.filters.len()
        {
            model.builder.filter_selected = model.config.filters.len().saturating_sub(1);
        }
    }
}

async fn refresh(model: &mut AppModel, debug: bool) {
    model.status = "Scanning...".into();
    model.error = None;
    match run_scan(&model.config, debug).await {
        Ok(rows) => match model.config.resolve_fields() {
            Ok(fields) => model.set_rows(fields, rows),
            Err(err) => model.set_error(err.to_string()),
        },
        Err(err) => model.set_error(err.to_string()),
    }
}

fn copy_active_pane(model: &mut AppModel) {
    let payload = match model.view {
        ViewMode::Results => serde_json::to_string_pretty(&model.rows).map_err(|e| e.to_string()),
        ViewMode::Payload => payload_pretty(&model.config, false).map_err(|e| e.to_string()),
        ViewMode::Codegen => Ok(render_codegen(&model.config)),
        ViewMode::Builder | ViewMode::Help => {
            model.status = "copy on Results, Payload, or Codegen".into();
            return;
        }
    };
    match payload {
        Ok(text) => match copy_via_osc52(&text) {
            Ok(()) => model.status = "copied to clipboard (OSC 52)".into(),
            Err(err) => model.set_error(format!("clipboard write failed: {err}")),
        },
        Err(err) => model.set_error(format!("encode failed: {err}")),
    }
}

async fn run_scan(config: &ScanConfig, debug: bool) -> Result<Vec<ScreenerRow>> {
    Ok(tvscreener::core::get_for_asset(config.asset, |inner| {
        configure_screener(inner, config, debug)
    })
    .await?)
}
