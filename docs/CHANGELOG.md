# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Dependencies

- Bumped crates to current crates.io releases (including `reqwest` 0.13)
- MCP server migrated from `mcpkit` to `rmcp` 3.1 (stdio + Streamable HTTP)

### CLI

- Interactive prompt: ↑/↓ recall prior commands (via rustyline); history persisted in `~/.tvscreener_history`

## [1.1.0] - 2026-08-02

### Packaging

- Default features are **lean**: path/git library consumers no longer pull `mcpkit`, `ratatui`, `crossterm`, `ctrlc`, `clap`, `anyhow`, or `tracing-subscriber`
- New features: `cli`, `mcp`, `tui`, and meta-feature `apps` (`cli` + `mcp` + `tui`)
- Binaries require their features (`tvscreener` → `cli`, `tvscreener-mcp` → `mcp`, `tvscreener-tui` → `tui`)
- Make defaults to `--features apps`; `make check-lib` verifies the lean graph
- Docker / CI / release builds pass `--features apps`
- Install apps: `cargo install --path . --features apps` (plain `cargo install --path .` no longer builds MCP/TUI bins)

## [1.0.0] - 2026-07-31

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
- No subcommand: interactive prompt until `quit` / `exit` / EOF

### MCP (`tvscreener-mcp`)

- Stdio MCP server with discover, custom query, search helpers, catalog tools, payload preview
- Stdio transport by default (`make run-mcp`)
- Optional Streamable HTTP (`--http`, default listen `0.0.0.0:6790`)

### Docker

- Image ships `tvscreener`, `tvscreener-mcp`, `tvscreener-tui`, and `tvscreener-entrypoint`
- Runtime: distroless `cc-debian13` (Debian 13); builder `rust:slim-trixie` (no bookworm, no Alpine)
- Entrypoint is a Rust binary (no shell) so Hub Scout is not flooded by perl/tar OS packages
- Default attached run: TUI + MCP HTTP on **6790**; `-d` for MCP only; CLI overrides
- Release attaches the three user-facing Linux binaries

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
- Install from git / path / Docker (not crates.io)
