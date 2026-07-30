# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Unofficial Rust [TradingView](https://www.tradingview.com) Screener HTTP client
(inspired by [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener)).
Not affiliated with TradingView.

### Library

- Six typed screeners: stock, crypto, forex, bond, futures, coin
- Filters (`FieldCondition` / `FilterOperator`), field catalog and presets, stream helpers
- Field-aware `format_cell` / `format_rows_table` for terminal display
- Shared `query_config` for filter/sort parse and apply (CLI, TUI, MCP)
- Shared `resolve` wire helpers (markets, indices, sectors, …)
- `core::with_asset_screener` / `get_for_asset` typed screener dispatch

### CLI (`tvscreener`)

- `scan` and `payload` (offline request JSON)
- Catalog commands: presets, fields, markets, sectors
- Scan output: `--format table|row|json`, `--json`, `--color`
- Query flags: `--filter`, `--filters`, `--sort-by`, `--ascending`, `--markets`, `--index`, `--search`

### MCP (`tvscreener-mcp`)

- Stdio MCP server with discover, custom query, search helpers, catalog tools, payload preview

### TUI (`tvscreener-tui`)

- Results, Builder (asset, preset, limit, search, sort, markets/index, filters), Payload, Codegen
- Opt-in watch refresh, OSC 52 clipboard copy
- Launch flags mirror CLI filters and sort

### Docs and tooling

- Guides (getting started, filtering, TUI, streaming, MCP)
- CI: `dtolnay/rust-toolchain`, `make deny`, release artifacts for all three binaries
- Maintainer `regen-fields` (`--features regen`)

### Notes

- `get()` returns `Vec<ScreenerRow>` (not a DataFrame)
- MSRV 1.85
