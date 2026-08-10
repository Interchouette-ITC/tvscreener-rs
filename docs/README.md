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
- **Terminal formatting**: field-aware `format_cell` / `format_rows_table` (K/M/B, %, ratings)
- **CLI** `tvscreener` (`scan` table/json/row, `payload`, catalog commands)
- **TUI** `tvscreener-tui`: results, builder, payload JSON, codegen
- **MCP** `tvscreener-mcp` for AI assistants

## Quick start

### Library

```toml
# Cargo.toml (path or git)
tvscreener = { git = "https://github.com/Interchouette-ITC/tvscreener-rs", branch = "dev" }
# or: tvscreener = { path = "../tvscreener-rs" }
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

### Install binaries

```bash
cargo install --path . --features apps   # installs tvscreener, tvscreener-mcp, tvscreener-tui
```

From a checkout without installing, use `make run` / `make run-mcp` / `make run-tui` (default `--features apps`) or `cargo run --features apps --bin …`.

### CLI

```bash
tvscreener --help
tvscreener                              # interactive prompt (quit / exit to leave)
tvscreener payload crypto --limit 2     # print request JSON only
tvscreener scan crypto --limit 5        # live HTTP (table)
tvscreener scan crypto --limit 5 --json
tvscreener scan stock --preset stock_price --index SP500 --limit 10 --color always
tvscreener scan stock --filter close:greater:100 --sort-by volume
tvscreener payload stock --filters '[{"field":"close","op":">","value":100}]'

# checkout shortcuts:
make run                                # --help
make run ARGS='scan crypto --limit 5'
cargo run --features apps --bin tvscreener -- scan crypto --limit 5
```

### TUI

Interactive Ratatui pane: Results table, Builder (asset / preset / limit / search / sort / markets / index / filters), Payload JSON, and Codegen (Rust + CLI).

```bash
tvscreener-tui --help
tvscreener-tui crypto --preset crypto_price --limit 10

# checkout shortcuts:
make run-tui
make run-tui ARGS='crypto --preset crypto_price --limit 10'
cargo run --features apps --bin tvscreener-tui -- crypto --preset crypto_price --limit 10
```

Keys (short): `Tab` / `1`–`4` switch views · `←`/`→` asset/preset · `r` refresh · `a` watch · `c` copy · `h` help · `q` quit.

Full key map and Builder details: [`guide/tui.md`](guide/tui.md).

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
tvscreener-mcp --help
tvscreener-mcp                          # stdio (default; Cursor / local clients)
tvscreener-mcp --http                   # Streamable HTTP on 0.0.0.0:6790
tvscreener-mcp --http --listen 127.0.0.1:6790

# checkout shortcuts:
make run-mcp
cargo run --features apps --bin tvscreener-mcp
```

**Tools include:** `discover_fields`, `custom_query`, `search_stocks` / `search_crypto` / `search_forex`, `get_top_movers`, `list_presets` / `get_preset`, plus catalog helpers (`list_markets`, `list_sectors`, `list_countries`, `list_industries`, `list_exchanges`, `list_ratings`, `list_filter_operators`, `list_index_symbols`, `build_payload`, `search_by_index`, …).

Docker image (CLI + TUI + MCP HTTP on **6790**), public pulls:

- Docker Hub: [`interchouette/tvscreener-rs`](https://hub.docker.com/r/interchouette/tvscreener-rs)
- GHCR: [`ghcr.io/interchouette/tvscreener-rs`](https://github.com/Interchouette?tab=packages)
- GHCR: [`ghcr.io/interchouette-itc/tvscreener-rs`](https://github.com/orgs/Interchouette-ITC/packages)

`gregoshop/tvscreener-rs` is a **deprecated Hub mirror** and is **no longer published**.

```bash
docker pull interchouette/tvscreener-rs:dev
docker run -d -p 6790:6790 interchouette/tvscreener-rs:dev          # MCP HTTP only
docker run -it -p 6790:6790 interchouette/tvscreener-rs:dev         # TUI + MCP
docker run --rm interchouette/tvscreener-rs:dev tvscreener --help   # one-shot CLI
```

Details and tags: [`docker/README.md`](../docker/README.md).

```bash
make docker-build-dev && make docker-push-dev   # :dev when you want (local)
# GitHub Actions → "CI/CD Image dev" (workflow_dispatch) for :dev
# GitHub Release tag vX.Y.Z → pushes :X.Y.Z and :latest (+ binaries)
make version-show
```

## Documentation

| Guide | Description |
| ----- | ----------- |
| [`OVERVIEW.md`](OVERVIEW.md) | Overview, build, Docker, module map |
| [Quick start](getting-started/quickstart.md) | First scan in a few minutes |
| [Filtering](guide/filtering.md) | Operators, conditions, merge rules |
| [Selecting fields](guide/selecting-fields.md) | Columns, presets, `select_all` |
| [Streaming](guide/streaming.md) | Periodic `stream` polls |
| [TUI](guide/tui.md) | Ratatui: Results, Builder, Payload, Codegen |
| [MCP](guide/mcp.md) | Stdio and Streamable HTTP (`tvscreener-mcp`) |
| [Screeners](guide/screeners.md) | All six typed clients |
| [Manual test plan](MANUAL_TEST_PLAN.md) | How to run tests |
| [`CHANGELOG.md`](CHANGELOG.md) | Semver notes |
| `make doc` → rustdoc | [`api-rust/tvscreener/`](api-rust/tvscreener/index.html) |

## Build and verify

Rust **1.85+**.

```bash
make test          # default suite (offline)
make lint
make verify        # format-check + clippy + tests
make test-live     # against TradingView (TVSCREENER_LIVE=1)
make doc           # API HTML under docs/api-rust/
make run           # CLI (ARGS=…, default --help)
make run-mcp       # MCP stdio server
make run-tui       # TUI (ARGS=…, default --help)
make help
```

## Origin

Rust port of [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener). Same unofficial TradingView scanner HTTP surface; this crate adds a first-class CLI, MCP server, and Ratatui TUI. See [License](#license).

## License

[Apache-2.0](../LICENSE). Based on [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener).
