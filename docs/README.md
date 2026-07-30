# tvscreener-rs

This repository is a **Rust port and translation** of the Python library **[deepentropy/tvscreener](https://github.com/deepentropy/tvscreener)** by [deepentropy](https://github.com/deepentropy). It is **not** a TradingView product and **is not affiliated with TradingView**. It uses the same unofficial scanner HTTP endpoints and JSON payload ideas as the Python client. Your use of TradingView data remains subject to [TradingView’s terms](https://www.tradingview.com/policies/).

| What                     | Where                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| **Upstream (Python)**    | [github.com/deepentropy/tvscreener](https://github.com/deepentropy/tvscreener)           |
| **Python documentation** | [deepentropy.github.io/tvscreener/docs/](https://deepentropy.github.io/tvscreener/docs/) |
| **This crate (Rust)**    | **tvscreener-rs** - Rust port (Docker Hub: `gregoshop/tvscreener-rs`)                    |

## License and attribution

- **License:** [Apache-2.0](../LICENSE).
- Upstream’s `pyproject.toml` may list MIT; for **this port**, treat **Apache-2.0** as authoritative.
- API shape, field names, and scanner behavior are derived from **deepentropy/tvscreener**; please cite that project when referring to the original design.

## Features

- **6 screener types**: Stock, Crypto, Forex, Bond, Futures, and Coin
- **Filter builders**: `FieldCondition` + `FilterOperator` (same wire ops as the Python client)
- **Fields & presets**: `FieldDef`, `get_preset` / `list_presets`, curated defaults
- **Async HTTP**: `get()` / `stream()` over `reqwest` + rustls
- **Terminal formatting**: field-aware `format_cell` / `format_rows_table` (and `format_value` / `format_row`)
- **Results**: `Vec<ScreenerRow>` (`symbol` + label-keyed `data` map)
- **TUI**: `tvscreener-tui` Ratatui results pane

## Quick example

```rust
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::field::get_preset;
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut screener = CryptoScreener::new();
    screener.select(get_preset("crypto_price")?).set_range(0, 5);
    let rows = screener.get().await?;
    for row in rows {
        println!("{} {:?}", row.symbol, row.data.get("Price"));
    }
    Ok(())
}
```

## Documentation map

| Section                                              | Description                                    |
| ---------------------------------------------------- | ---------------------------------------------- |
| [Installation](getting-started/installation.md)      | Add the crate / build from source              |
| [Quick start](getting-started/quickstart.md)         | First scan in a few minutes                    |
| [Filtering](guide/filtering.md)                      | Operators, conditions, merge rules             |
| [Selecting fields](guide/selecting-fields.md)        | Columns, presets, `select_all`                 |
| [Sorting & range](guide/sorting-pagination.md)       | `sort_by`, `set_range`                         |
| [Streaming](guide/streaming.md)                      | Periodic `stream` polls                        |
| [TUI](guide/tui.md)                                  | `tvscreener-tui` views, builder, payload       |
| [Screeners](guide/screeners.md)                      | Stock / crypto / forex / bond / futures / coin |
| [TUI](guide/tui.md)                                  | `tvscreener-tui` views, builder, payload       |
| [Examples](examples/crypto.md)                       | Walkthrough of `example_crypto`                |
| [Manual test plan](MANUAL_TEST_PLAN.md)              | How to run tests                               |
| [API overview](api/overview.md)                      | Modules and main types                         |
| [Rust API (rustdoc)](api-rust/tvscreener/index.html) | Generated with `make doc`                      |
| [Changelog](changelog.md)                            | Version history                                |

## What this crate provides

Typed scanner clients (`StockScreener`, `CryptoScreener`, …), an embedded field catalog, tests (default and optional live HTTP), CLI **`tvscreener`**, TUI **`tvscreener-tui`**, and MCP **`tvscreener-mcp`**.

## Depending on this crate

Package name in `Cargo.toml` is **`tvscreener`** (repo folder may be `tvscreener-rs`). Prefer a path or git dependency until the crate is published on crates.io:

```toml
tvscreener = { git = "https://github.com/Interchouette-ITC/tvscreener-rs", branch = "dev" }
# or: tvscreener = { path = "../tvscreener-rs" }
# after crates.io publish: tvscreener = "1.0"
```

**This crate’s Cargo features:**

| Feature     | What it enables                          |
| ----------- | ---------------------------------------- |
| _(default)_ | Library + CLI + MCP binaries + tests     |
| `live`      | Live HTTP tests (`make test-live`)       |
| `regen`     | `tvscreener regen-fields` maintainer cmd |
| `tui`       | Ratatui binary `tvscreener-tui`          |

Scanner POSTs happen when you call `get()` / `stream()` (or `tvscreener scan` / `tvscreener-tui`) at runtime.

### Logging / debug

Library emits [`tracing`](https://docs.rs/tracing) events. Both binaries call `init_logging()`:

```bash
TVSCREENER_DEBUG=1 tvscreener scan crypto --limit 3
RUST_LOG=tvscreener=debug,info tvscreener-mcp
```

Per-screener debug (URL + payload at DEBUG): `screener.set_debug(true)` (alias: `set_print_request(true)`).

Errors: library uses **`thiserror`** (`TvscreenerError`); binaries use **`anyhow`** at the process edge.

This repository does **not** ship a browser UI or host-app HTTP routes. Terminal UI: `make run-tui` / `tvscreener-tui`. MCP is a separate stdio binary (`make run-mcp`).

## Results

`get()` returns **`Vec<ScreenerRow>`**: each row has a `symbol` string and a `data` map keyed by **column label** → JSON value.

Terminal helpers: `format_cell` / `format_rows_table` (field-aware), plus `util::format_value` / `util::format_row`.

## Build and verify

Rust **1.85+** (`rust-version` in `Cargo.toml`; required by `mcpkit`).

```bash
make test          # default test suite
make lint          # fmt --check + clippy -D warnings
make verify        # format-check + clippy + tests
make test-live     # against TradingView (TVSCREENER_LIVE=1)
make doc           # rustdoc → docs/api-rust/
make help          # all targets
```

Or raw cargo:

```bash
cargo build
cargo test
cargo check --bins --examples
cargo doc --no-deps --open
```

## Docker (Hub + GHCR)

Public pulls: Docker Hub `gregoshop/tvscreener-rs`, `ghcr.io/interchouette/tvscreener-rs`, `ghcr.io/interchouette-itc/tvscreener-rs`.

```bash
make docker-build-dev && make docker-push-dev   # :dev on demand
make version-show                               # suggested GitHub Release tag vX.Y.Z
```

Entrypoint is `tvscreener-mcp` (stdio MCP). Tags `:dev` (manual), `:X.Y.Z` + `:latest` (GitHub Release).

Details: [`docker/README.md`](../docker/README.md).

## Examples

```bash
cargo run --example manual_test
cargo run --example example_crypto    # POST to scanner.tradingview.com
```

`example_crypto` requires network. See [MANUAL_TEST_PLAN.md](MANUAL_TEST_PLAN.md).

## Tests

| Kind        | Command                                                          |
| ----------- | ---------------------------------------------------------------- |
| **Default** | `make test` / `cargo test`                                       |
| **Live**    | `make test-live` (`TVSCREENER_LIVE=1`, `--features live`)        |
| **Verify**  | `make verify` (fmt + clippy + tests)                             |

Default suite does not call the scanner. Live tests need the `live` feature and `TVSCREENER_LIVE=1`.

## MCP binary (`mcpkit`)

```bash
cargo run --bin tvscreener-mcp
# or: make run-mcp / tvscreener-mcp after cargo install --path .
```

Tools: `discover_fields`, `list_field_types`, `custom_query`, `search_stocks` / `search_crypto` / `search_forex`, `get_top_movers`, `list_presets`, `get_preset`, `list_sectors`, `list_countries`, `list_industries`, `list_exchanges`, `list_ratings`, `list_filter_operators`, `list_markets`, `list_index_symbols`, `build_payload`, `search_by_index`.

## Module layout

```
src/
  lib.rs           ScreenerRow, re-exports
  error.rs         TvscreenerError
  filter.rs        FilterOperator, ExtraFilter, Filter, FieldCondition
  util.rs          get_url, headers, millify, format_value / format_row
  beautify.rs      format_cell / format_rows_table (field-aware tones)
  ta.rs            ADX / AO / Bollinger helpers for computed recommendations
  logging.rs       env_debug_enabled; init_logging
  core/            Screener builder + Stock/Crypto/Forex/Bond/Futures/Coin
  field/           FieldDef (label, field_name, format, interval, historical)
  mcp/             tools + mcpkit stdio server
  tui/             Ratatui pane (`tvscreener-tui`)
  bin/tvscreener/main.rs
  bin/tvscreener_mcp.rs
  bin/tvscreener_tui.rs
  resolve.rs       Shared market/index/sector wire resolution
examples/
tests/
docs/
docker/
```

## CLI (`tvscreener`) / MCP (`tvscreener-mcp`)

```bash
cargo install --path .
tvscreener --help
tvscreener scan crypto --limit 5
tvscreener scan stock --preset stock_price --index SP500 --limit 10
tvscreener-mcp
```

Checkout without install: `cargo run -- …` / `cargo run --bin tvscreener-mcp`. Dev shortcuts: `make run` / `make run-mcp` (see `make help`).

CLI checks: `tests/cli.rs` (included in `make test`).
