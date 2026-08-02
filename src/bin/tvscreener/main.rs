// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener` - command-line client for the library.
//!
//! ```bash
//! cargo run --features apps --bin tvscreener -- --help
//! cargo run --features apps --bin tvscreener -- scan crypto --limit 5
//! cargo run --features apps --bin tvscreener -- payload stock --index SP500
//! TVSCREENER_DEBUG=1 cargo run --features apps --bin tvscreener -- scan stock --limit 3
//! cargo run --features apps,regen --bin tvscreener -- regen-fields --python-root ../tvscreener
//! # no subcommand: interactive prompt until quit / exit / EOF
//! cargo run --features apps --bin tvscreener
//! ```

mod output;
#[cfg(feature = "regen")]
mod regen;

use std::io::{self, Write};

use anyhow::Result;
use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use tvscreener::core::Screener;
use tvscreener::field::{
    all_markets, all_sectors, default_fields, get_preset, list_presets, search_fields, Asset,
    FieldDef,
};
use tvscreener::query_config::{
    apply_filters, apply_filters_json, apply_sort, parse_filter_token, parse_filters_arg,
};
use tvscreener::resolve::apply_stock_index_markets;
use tvscreener::ScreenerRow;

use output::{print_scan_rows, ColorWhen, ScanFormat};

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
    name = "tvscreener",
    about = "TradingView screener HTTP client (library + CLI)",
    after_help = "With no subcommand, starts an interactive prompt (type quit or exit to leave).",
    subcommand_required = false,
    arg_required_else_help = false,
    version
)]
struct Cli {
    /// Force screener debug logs (also: `TVSCREENER_DEBUG=1`).
    #[arg(long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// POST to scanner.tradingview.com and print rows.
    Scan(ScanArgs),
    /// Print request JSON only (no network).
    Payload(ScanArgs),
    /// List preset names.
    Presets,
    /// Search field catalog.
    Fields {
        query: String,
        #[arg(long, value_enum, default_value_t = AssetArg::Stock)]
        asset: AssetArg,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// List stock markets (const → wire).
    Markets,
    /// List sectors (const → wire).
    Sectors,
    /// Regenerate `data/fields.json` + `src/field/generated/` from a Python tvscreener clone.
    #[cfg(feature = "regen")]
    RegenFields {
        /// Path to a deepentropy/tvscreener checkout (also: `TVSCREENER_PYTHON_ROOT`).
        #[arg(long, env = "TVSCREENER_PYTHON_ROOT")]
        python_root: std::path::PathBuf,
        /// Crate root containing `data/` and `src/field/generated/` (default: cwd).
        #[arg(long, default_value = ".")]
        crate_root: std::path::PathBuf,
        /// Output JSON path (default: `<crate-root>/data/fields.json`).
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone, clap::Args)]
struct ScanArgs {
    asset: AssetArg,
    /// Preset name (see `tvscreener presets`).
    #[arg(long)]
    preset: Option<String>,
    /// Result window start (inclusive).
    #[arg(long, default_value_t = 0)]
    from: u32,
    /// Max rows (window end = from + limit).
    #[arg(long, default_value_t = 10)]
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
    /// Output shape for `scan` (ignored by `payload`).
    #[arg(long, value_enum, default_value_t = ScanFormat::Table)]
    format: ScanFormat,
    /// Shorthand for `--format json` (scan only).
    #[arg(long)]
    json: bool,
    /// ANSI colors for `--format table` cells.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
    color: ColorWhen,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tvscreener::logging::init_logging();

    match cli.command {
        Some(command) => run_command(command, cli.debug).await,
        None => run_interactive(cli.debug).await,
    }
}

async fn run_interactive(debug: bool) -> Result<()> {
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "tvscreener interactive mode - enter a subcommand (scan, payload, ...); quit or exit to leave"
    )?;
    stdout.flush()?;

    loop {
        write!(stdout, "tvscreener> ")?;
        stdout.flush()?;
        let mut line = String::new();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            writeln!(stdout)?;
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed.to_ascii_lowercase().as_str(), "quit" | "exit" | "q") {
            break;
        }
        if matches!(trimmed, "help" | "--help" | "-h") {
            let _ = Cli::command().print_help();
            writeln!(stdout)?;
            continue;
        }

        let mut argv = vec!["tvscreener".to_string()];
        argv.extend(trimmed.split_whitespace().map(str::to_string));
        match Cli::try_parse_from(&argv) {
            Ok(parsed) => match parsed.command {
                Some(command) => {
                    if let Err(err) = run_command(command, debug || parsed.debug).await {
                        eprintln!("{err:#}");
                    }
                }
                None => {
                    eprintln!("enter a subcommand (e.g. scan crypto --limit 5), or quit");
                }
            },
            Err(err) => {
                // Clap already formats help / usage; print without exiting the prompt.
                eprint!("{err}");
            }
        }
    }
    Ok(())
}

async fn run_command(command: Commands, debug: bool) -> Result<()> {
    match command {
        Commands::Scan(args) => {
            let fields = resolve_fields(args.asset, args.preset.as_deref())?;
            let rows = run_scan(&args, debug).await?;
            print_scan_rows(&rows, &fields, args.format, args.color, args.json)?;
            if !rows.is_empty() {
                tracing::info!(count = rows.len(), "scan done");
            }
        }
        Commands::Payload(args) => {
            let payload = build_payload_only(&args, debug)?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        Commands::Presets => {
            for name in list_presets() {
                println!("{name}");
            }
        }
        Commands::Fields {
            query,
            asset,
            limit,
        } => {
            let hits = search_fields(asset.asset(), &query);
            for def in hits.into_iter().take(limit.max(1)) {
                println!("{}  ({})", def.field_name, def.label);
            }
        }
        Commands::Markets => {
            for m in all_markets() {
                println!("{}  {}", m.const_name, m.value);
            }
        }
        Commands::Sectors => {
            for s in all_sectors() {
                println!("{}  {}", s.const_name, s.value);
            }
        }
        #[cfg(feature = "regen")]
        Commands::RegenFields {
            python_root,
            crate_root,
            out,
        } => {
            let out = out.unwrap_or_else(|| regen::default_out(&crate_root));
            regen::run(&python_root, &crate_root, &out)?;
        }
    }
    Ok(())
}

fn resolve_fields(asset: AssetArg, preset: Option<&str>) -> tvscreener::Result<Vec<FieldDef>> {
    if let Some(name) = preset {
        return get_preset(name);
    }
    Ok(default_fields(asset.asset()))
}

fn configure_inner(inner: &mut Screener, args: &ScanArgs, debug: bool) -> tvscreener::Result<()> {
    let asset = args.asset.asset();
    let fields = resolve_fields(args.asset, args.preset.as_deref())?;
    if debug || tvscreener::logging::env_debug_enabled() {
        inner.set_debug(true);
    }
    inner
        .select(fields)
        .set_range(args.from, args.from.saturating_add(args.limit));
    if let Some(q) = args.search.as_deref() {
        inner.search(q)?;
    }
    let mut conditions = Vec::with_capacity(args.filter.len());
    for token in &args.filter {
        conditions.push(parse_filter_token(token)?);
    }
    apply_filters(inner, asset, &conditions)?;
    let json_filters = parse_filters_arg(args.filters.as_deref())?;
    apply_filters_json(inner, asset, &json_filters)?;
    apply_sort(inner, asset, args.sort_by.as_deref(), args.ascending)?;
    if matches!(args.asset, AssetArg::Stock) {
        apply_stock_index_markets(inner, args.index.as_deref(), args.markets.as_deref())?;
    } else if args.markets.is_some() || args.index.is_some() {
        return Err(tvscreener::TvscreenerError::InvalidRequest(
            "--markets / --index only apply to stock".into(),
        ));
    }
    Ok(())
}

fn build_payload_only(args: &ScanArgs, debug: bool) -> Result<serde_json::Value> {
    Ok(tvscreener::core::with_asset_screener(
        args.asset.asset(),
        |inner| {
            configure_inner(inner, args, debug)?;
            inner.build_payload()
        },
    )?)
}

async fn run_scan(args: &ScanArgs, debug: bool) -> Result<Vec<ScreenerRow>> {
    Ok(
        tvscreener::core::get_for_asset(args.asset.asset(), |inner| {
            configure_inner(inner, args, debug)
        })
        .await?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subcommand_still_required_shape() {
        let cli = Cli::try_parse_from(["tvscreener", "presets"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Presets)));
    }

    #[test]
    fn parse_no_subcommand_is_interactive() {
        let cli = Cli::try_parse_from(["tvscreener"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn interactive_line_parses_scan() {
        let argv = ["tvscreener", "scan", "crypto", "--limit", "2"];
        let cli = Cli::try_parse_from(argv).unwrap();
        match cli.command {
            Some(Commands::Scan(scan_args)) => assert_eq!(scan_args.limit, 2),
            other => panic!("expected Scan, got {other:?}"),
        }
    }
}
