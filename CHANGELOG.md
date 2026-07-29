# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-07-29

Initial release: Rust port of [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener).

- Library: six typed screeners, filters, field catalog/presets, stream, display helpers
- CLI: `tvscreener` (`scan`, `payload`, catalog commands)
- Optional MCP: `tvscreener-mcp` (`--features mcp`)
- `get()` returns `Vec<ScreenerRow>` (not a DataFrame)
- MSRV 1.85
