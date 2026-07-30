// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener` - command-line client for the library.
//!
//! ```bash
//! cargo run --bin tvscreener -- --help
//! cargo run --bin tvscreener -- scan crypto --limit 5
//! cargo run --bin tvscreener -- payload stock --index SP500
//! TVSCREENER_DEBUG=1 cargo run --bin tvscreener -- scan stock --limit 3
//! cargo run --features regen --bin tvscreener -- regen-fields --python-root ../tvscreener
//! ```

#[cfg(feature = "regen")]
mod regen;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tvscreener::core::Screener;
use tvscreener::field::{
    all_markets, all_sectors, default_fields, get_preset, list_presets, search_fields, Asset,
    FieldDef,
};
use tvscreener::resolve::apply_stock_index_markets;
use tvscreener::util::format_row;
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
    name = "tvscreener",
    about = "TradingView screener HTTP client (library + CLI)",
    version
)]
struct Cli {
    /// Force screener debug logs (also: `TVSCREENER_DEBUG=1`).
    #[arg(long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tvscreener::logging::init_logging();

    match cli.command {
        Commands::Scan(args) => {
            let rows = run_scan(&args, cli.debug).await?;
            if rows.is_empty() {
                println!("(no rows)");
            } else {
                for row in &rows {
                    println!("{}", format_row(row, None));
                }
                tracing::info!(count = rows.len(), "scan done");
            }
        }
        Commands::Payload(args) => {
            let payload = build_payload_only(&args, cli.debug)?;
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
