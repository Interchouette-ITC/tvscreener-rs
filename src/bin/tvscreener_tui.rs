// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener-tui` - Ratatui results pane (feature `tui`).
//!
//! ```bash
//! cargo run --features tui --bin tvscreener-tui -- crypto --limit 10
//! make run-tui ARGS='crypto --preset crypto_price --limit 5'
//! make run-tui ARGS='crypto --watch --interval 30'
//! ```

use std::io::{self, stdout};
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tvscreener::core::Screener;
use tvscreener::field::Asset;
use tvscreener::resolve::apply_stock_index_markets;
use tvscreener::tui::{
    copy_via_osc52, draw, hard_reset_tty, inside_gnu_screen, install_panic_hook,
    install_signal_handlers, is_quit_key, AppModel, ScanConfig, TerminalGuard,
    DEFAULT_WATCH_INTERVAL_SECS, MIN_TUI_REFRESH_SECS, STOP,
};
use tvscreener::ScreenerRow;

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
    about = "TradingView screener Ratatui results pane",
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

    let config = ScanConfig {
        asset: cli.asset.asset(),
        preset: cli.preset.clone(),
        from: cli.from,
        limit: cli.limit,
        search: cli.search.clone(),
        markets: cli.markets.clone(),
        index: cli.index.clone(),
    };

    let interval = cli.interval.max(MIN_TUI_REFRESH_SECS);

    // Scan before taking the tty so HTTP noise never paints into the UI.
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

        match key.code {
            KeyCode::Char('h') => model.toggle_help(),
            KeyCode::Char('a') => model.toggle_watch(),
            KeyCode::Char('c') => copy_rows(model),
            KeyCode::Char('r') => {
                if model.can_refresh() {
                    refresh(model, debug).await;
                } else {
                    let wait = model.cooldown_secs_remaining();
                    model.status = format!("cooldown {wait}s (min {MIN_TUI_REFRESH_SECS}s)");
                }
            }
            KeyCode::Up => model.scroll_by(-1),
            KeyCode::Down => model.scroll_by(1),
            KeyCode::PageUp => model.scroll_by(-10),
            KeyCode::PageDown => model.scroll_by(10),
            _ => {}
        }
    }
    Ok(())
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

fn copy_rows(model: &mut AppModel) {
    match serde_json::to_string_pretty(&model.rows) {
        Ok(json) => match copy_via_osc52(&json) {
            Ok(()) => {
                model.status = format!("copied {} rows (OSC 52)", model.rows.len());
            }
            Err(err) => model.set_error(format!("clipboard write failed: {err}")),
        },
        Err(err) => model.set_error(format!("json encode failed: {err}")),
    }
}

async fn run_scan(config: &ScanConfig, debug: bool) -> Result<Vec<ScreenerRow>> {
    Ok(
        tvscreener::core::get_for_asset(config.asset, |inner| {
            configure_inner(inner, config, debug)
        })
        .await?,
    )
}

fn configure_inner(
    inner: &mut Screener,
    config: &ScanConfig,
    debug: bool,
) -> tvscreener::Result<()> {
    let fields = config.resolve_fields()?;
    if debug || tvscreener::logging::env_debug_enabled() {
        inner.set_debug(true);
    }
    inner
        .select(fields)
        .set_range(config.from, config.from.saturating_add(config.limit));
    if let Some(q) = config.search.as_deref() {
        inner.search(q)?;
    }
    if matches!(config.asset, Asset::Stock) {
        apply_stock_index_markets(inner, config.index.as_deref(), config.markets.as_deref())?;
    } else if config.markets.is_some() || config.index.is_some() {
        return Err(tvscreener::TvscreenerError::InvalidRequest(
            "--markets / --index only apply to stock".into(),
        ));
    }
    Ok(())
}
