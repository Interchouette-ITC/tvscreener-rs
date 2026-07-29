// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! `tvscreener` - command-line client for the library.
//!
//! ```bash
//! cargo run --bin tvscreener -- --help
//! cargo run --bin tvscreener -- scan crypto --limit 5
//! cargo run --bin tvscreener -- payload stock --index SP500
//! TVSCREENER_DEBUG=1 cargo run --bin tvscreener -- scan stock --limit 3
//! cargo run --bin tvscreener -- regen-fields --python-root ../tvscreener
//! ```

mod regen;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tvscreener::core::bond::BondScreener;
use tvscreener::core::coin::CoinScreener;
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::core::forex::ForexScreener;
use tvscreener::core::futures::FuturesScreener;
use tvscreener::core::stock::StockScreener;
use tvscreener::core::Screener;
use tvscreener::field::{
    all_index_symbols, all_markets, all_sectors, get_preset, index_symbol_value, list_presets,
    market, search_fields, Asset, FieldDef,
};
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
    /// Live scan (POST to scanner.tradingview.com).
    Scan(ScanArgs),
    /// Print scan JSON payload without HTTP.
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
    RegenFields {
        /// Path to a deepentropy/tvscreener checkout (also: `TVSCREENER_PYTHON_ROOT`).
        #[arg(long, env = "TVSCREENER_PYTHON_ROOT")]
        python_root: PathBuf,
        /// Crate root containing `data/` and `src/field/generated/` (default: cwd).
        #[arg(long, default_value = ".")]
        crate_root: PathBuf,
        /// Output JSON path (default: `<crate-root>/data/fields.json`).
        #[arg(long)]
        out: Option<PathBuf>,
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

fn resolve_fields(asset: AssetArg, preset: Option<&str>) -> Result<Vec<FieldDef>> {
    if let Some(name) = preset {
        return get_preset(name).with_context(|| format!("preset `{name}`"));
    }
    Ok(match asset {
        AssetArg::Stock => tvscreener::field::default_stock_fields(),
        AssetArg::Crypto => tvscreener::field::default_crypto_fields(),
        AssetArg::Forex => tvscreener::field::default_forex_fields(),
        AssetArg::Bond => tvscreener::field::default_bond_fields(),
        AssetArg::Futures => tvscreener::field::default_futures_fields(),
        AssetArg::Coin => tvscreener::field::default_coin_fields(),
    })
}

fn parse_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

fn resolve_markets_local(tokens: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for token in tokens {
        let upper = token.to_ascii_uppercase().replace(' ', "_");
        if let Some(m) = market(&upper) {
            out.push(m.value);
            continue;
        }
        if let Some(m) = all_markets()
            .iter()
            .find(|m| m.value.eq_ignore_ascii_case(token))
        {
            out.push(m.value.clone());
            continue;
        }
        bail!("unknown market `{token}` (try `tvscreener markets`)");
    }
    Ok(out)
}

fn resolve_indices_local(tokens: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for token in tokens {
        let catalog = token.strip_prefix("SYML:").unwrap_or(token.as_str());
        let upper = catalog.to_ascii_uppercase().replace(' ', "_");
        if let Some(wire) = index_symbol_value(&upper) {
            out.push(wire.to_string());
            continue;
        }
        if let Some(idx) = all_index_symbols()
            .iter()
            .find(|idx| idx.value.eq_ignore_ascii_case(catalog))
        {
            out.push(idx.value.clone());
            continue;
        }
        bail!("unknown index `{token}` (e.g. SP500)");
    }
    Ok(out)
}

fn apply_stock_extras(
    screener: &mut Screener,
    markets: Option<&str>,
    index: Option<&str>,
) -> Result<()> {
    let market_tokens = parse_csv(markets);
    if !market_tokens.is_empty() {
        screener.set_markets(resolve_markets_local(&market_tokens)?);
    }
    let index_tokens = parse_csv(index);
    if !index_tokens.is_empty() {
        screener.set_index(resolve_indices_local(&index_tokens)?);
    }
    Ok(())
}

fn configure_inner(inner: &mut Screener, args: &ScanArgs, debug: bool) -> Result<()> {
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
        apply_stock_extras(inner, args.markets.as_deref(), args.index.as_deref())?;
    } else if args.markets.is_some() || args.index.is_some() {
        bail!("--markets / --index only apply to stock");
    }
    Ok(())
}

fn build_payload_only(args: &ScanArgs, debug: bool) -> Result<serde_json::Value> {
    match args.asset {
        AssetArg::Stock => {
            let mut s = StockScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.inner().build_payload()?)
        }
        AssetArg::Crypto => {
            let mut s = CryptoScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.inner().build_payload()?)
        }
        AssetArg::Forex => {
            let mut s = ForexScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.inner().build_payload()?)
        }
        AssetArg::Bond => {
            let mut s = BondScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.inner().build_payload()?)
        }
        AssetArg::Futures => {
            let mut s = FuturesScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.inner().build_payload()?)
        }
        AssetArg::Coin => {
            let mut s = CoinScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.inner().build_payload()?)
        }
    }
}

async fn run_scan(args: &ScanArgs, debug: bool) -> Result<Vec<ScreenerRow>> {
    match args.asset {
        AssetArg::Stock => {
            let mut s = StockScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.get().await?)
        }
        AssetArg::Crypto => {
            let mut s = CryptoScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.get().await?)
        }
        AssetArg::Forex => {
            let mut s = ForexScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.get().await?)
        }
        AssetArg::Bond => {
            let mut s = BondScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.get().await?)
        }
        AssetArg::Futures => {
            let mut s = FuturesScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.get().await?)
        }
        AssetArg::Coin => {
            let mut s = CoinScreener::new();
            configure_inner(s.inner_mut(), args, debug)?;
            Ok(s.get().await?)
        }
    }
}
