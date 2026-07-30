# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Field-aware `format_cell` / `format_rows_table` for terminal display
- CLI `scan --format table|row|json`, `--json`, `--color`
- Optional TUI `tvscreener-tui` (`--features tui`): Results, Builder (preset, limit, search, filters), Payload JSON, Codegen, opt-in watch, OSC 52 copy
- Docs: [TUI guide](docs/guide/tui.md)

### Changed

- `tvscreener-mcp` always builds (no `--features mcp`); `mcpkit` is a normal dependency

## [1.0.0] - 2026-07-30

Initial release: Rust port of [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener).

### Added

- Library: six typed screeners, filters, field catalog/presets, stream, display helpers
- CLI: `tvscreener` (`scan`, `payload`, catalog commands)
- Optional MCP: `tvscreener-mcp` (`--features mcp`)
- Shared `crate::resolve` wire helpers (markets, indices, sectors, …); CLI and MCP one owner
- `core::with_asset_screener` / `get_for_asset` typed screener dispatch
- Crate-root re-exports: `resolve_field`, `catalog_len`
- CI: `dtolnay/rust-toolchain`, `make deny` (incl. `CDLA-Permissive-2.0`)
- Feature `regen` for maintainer `regen-fields` (optional `regex`)

### Changed

- `Cargo.toml` repository / homepage / authors → Interchouette-ITC
- MCP payload preview uses typed screener defaults
- `FilterOperator::from_wire` owns friendly op aliases; `TvscreenerError::Network` keeps `reqwest` source
- `format_rating` private; `field()` is `pub(crate)`

### Fixed

- Docs: git install path until crates.io; filtering guide; API overview; MANUAL_TEST_PLAN coverage map

### Notes

- `get()` returns `Vec<ScreenerRow>` (not a DataFrame)
- MSRV 1.85
