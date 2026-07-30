# tvscreener-rs

Unofficial **Rust** TradingView Screener HTTP client: Stock, Crypto, Forex, Bond, Futures, and Coin.

Port of [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener). **Not affiliated with TradingView.** Use is subject to [TradingView’s terms](https://www.tradingview.com/policies/).

## Features

- Query **Stock**, **Crypto**, **Forex**, **Bond**, **Futures**, and **Coin** screeners
- **13,000+ fields** embedded in the catalog (all time intervals included)
- **Fluent builders**: `select()`, `where_condition()`, `set_range()`, `search()`
- **Field discovery**: `search_fields`, `list_fields`, `resolve_field`
- **Field presets**: curated groups (`stock_valuation`, `crypto_price`, …)
- **Async HTTP**: `get()` / `stream()` via `reqwest` + rustls
- **Results**: `Vec<ScreenerRow>` (`symbol` + label-keyed `data` map)
- **Terminal formatting**: `format_value` / `format_row` (K/M/B, recommendations)
- **CLI** `tvscreener` (`scan`, `payload`, catalog commands)
- **MCP** `tvscreener-mcp` for AI assistants

## Quick start

### Library

```toml
# Cargo.toml (when published on crates.io)
tvscreener = "1.0"
# or from GitHub:
# tvscreener = { git = "https://github.com/Interchouette-ITC/tvscreener-rs", branch = "dev" }
```

```rust
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let rows = CryptoScreener::new().get().await?;
    println!("{} rows", rows.len());
    Ok(())
}
```

### CLI

```bash
cargo install --path .                  # installs `tvscreener` and `tvscreener-mcp`

tvscreener --help
tvscreener scan crypto --limit 5
tvscreener scan stock --preset stock_price --limit 10
tvscreener payload crypto --limit 2     # print request JSON only
tvscreener-mcp
```

From a checkout without installing: `cargo run -- …` / `cargo run --bin tvscreener-mcp`.

### All six screeners

```rust
use tvscreener::core::bond::BondScreener;
use tvscreener::core::coin::CoinScreener;
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::core::forex::ForexScreener;
use tvscreener::core::futures::FuturesScreener;
use tvscreener::core::stock::StockScreener;

let stocks = StockScreener::new().get().await?;
let crypto = CryptoScreener::new().get().await?;
let forex = ForexScreener::new().get().await?;
let bonds = BondScreener::new().get().await?;
let futures = FuturesScreener::new().get().await?;
let coins = CoinScreener::new().get().await?;
```

## Fluent API

```rust
use serde_json::json;
use tvscreener::core::stock::StockScreener;
use tvscreener::field::get_preset;
use tvscreener::filter::{FieldCondition, FilterOperator};
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut ss = StockScreener::new();
    ss.select(get_preset("stock_price")?)
        .where_condition(FieldCondition::new(
            "market_cap_basic",
            FilterOperator::Above,
            json!(1e9),
        ))?
        .where_condition(FieldCondition::new(
            "change", // Change %
            FilterOperator::Above,
            json!(5.0),
        ))?
        .set_range(0, 50);

    let rows = ss.get().await?;
    for row in &rows {
        println!("{} {:?}", row.symbol, row.data.get("Price"));
    }
    Ok(())
}
```

## Field discovery and presets

```rust
use tvscreener::field::{get_preset, list_presets, search_fields, Asset};

let rsi = search_fields(Asset::Stock, "rsi");
println!("{} RSI-related fields", rsi.len());

println!("{:?}", list_presets());
let fields = get_preset("stock_valuation")?;
```

| Category | Presets |
| -------- | ------- |
| Stock | `stock_price`, `stock_volume`, `stock_valuation`, `stock_dividend`, `stock_profitability`, `stock_performance`, `stock_oscillators`, `stock_moving_averages`, `stock_earnings` |
| Crypto | `crypto_price`, `crypto_volume`, `crypto_performance`, `crypto_technical` |
| Forex | `forex_price`, `forex_performance`, `forex_technical` |
| Bond | `bond_basic`, `bond_yield`, `bond_maturity` |
| Futures | `futures_price`, `futures_technical` |
| Coin | `coin_price`, `coin_market` |

Time-interval variants (1m, 5, 15, 30, 60, 120, 240, 1W, 1M, …) are separate catalog entries. Search for them (e.g. `search_fields(Asset::Stock, "RSI|60")`) or pick them from `list_fields`.

## Streaming

```rust
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let screener = CryptoScreener::new();
    // interval seconds, optional max iterations
    let snapshots = screener.inner().stream(10.0, Some(5)).await;
    println!("{} polls", snapshots.len());
    Ok(())
}
```

## MCP server (AI assistants)

```bash
cargo install --path .
tvscreener-mcp
# checkout: cargo run --bin tvscreener-mcp
```

**Tools include:** `discover_fields`, `custom_query`, `search_stocks` / `search_crypto` / `search_forex`, `get_top_movers`, `list_presets` / `get_preset`, plus catalog helpers (`list_markets`, `list_sectors`, `list_countries`, `list_industries`, `list_exchanges`, `list_ratings`, `list_filter_operators`, `list_index_symbols`, `build_payload`, `search_by_index`, …).

Docker image (stdio MCP), public pulls:

- Docker Hub: [`gregoshop/tvscreener-rs`](https://hub.docker.com/r/gregoshop/tvscreener-rs)
- GHCR: [`ghcr.io/interchouette/tvscreener-rs`](https://github.com/Interchouette?tab=packages)
- GHCR: [`ghcr.io/interchouette-itc/tvscreener-rs`](https://github.com/orgs/Interchouette-ITC/packages)

Details and tags: [`docker/README.md`](docker/README.md).

```bash
make docker-build-dev && make docker-push-dev   # :dev when you want (local)
# GitHub Actions → "CI/CD Image dev" (workflow_dispatch) for :dev
# GitHub Release tag vX.Y.Z → pushes :X.Y.Z and :latest (+ binaries)
make version-show
```

## Documentation

| Guide | Description |
| ----- | ----------- |
| [`docs/README.md`](docs/README.md) | Overview, build, Docker, module map |
| [Quick start](docs/getting-started/quickstart.md) | First scan in a few minutes |
| [Filtering](docs/guide/filtering.md) | Operators, conditions, merge rules |
| [Selecting fields](docs/guide/selecting-fields.md) | Columns, presets, `select_all` |
| [Streaming](docs/guide/streaming.md) | Periodic `stream` polls |
| [Screeners](docs/guide/screeners.md) | All six typed clients |
| [Manual test plan](docs/MANUAL_TEST_PLAN.md) | How to run tests |
| [`CHANGELOG.md`](CHANGELOG.md) | Semver notes |
| `make doc` → rustdoc | [`docs/api-rust/tvscreener/`](docs/api-rust/tvscreener/index.html) |

## Build and verify

Rust **1.85+**.

```bash
make test          # default suite (offline)
make lint
make verify        # format-check + clippy + tests
make test-live     # against TradingView (TVSCREENER_LIVE=1)
make doc           # API HTML under docs/api-rust/
make help
```

## Compared to the Python library

| Capability | Python `tvscreener` | This crate |
| ---------- | ------------------- | ---------- |
| Six screener types | Yes | Yes |
| ~13k fields + presets | Yes | Yes |
| Filters / markets / range | Yes | Yes |
| Streaming polls | Yes | Yes (async) |
| MCP server | Yes | Yes (+ extra catalog tools) |
| CLI | Limited | First-class `tvscreener` |
| Result type | Pandas `DataFrame` | `Vec<ScreenerRow>` |
| Filter sugar | `StockField.PRICE > 50` | `FieldCondition` + `FilterOperator` |
| Interval helper | `.with_interval("60")` | Pre-expanded catalog fields |
| Jupyter / styled HTML | `beautify` | Terminal `format_row` / `format_value` |
| Visual code generator | Web UI | No (CLI `payload` + docs) |

## License

[Apache-2.0](LICENSE). Based on [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener).
