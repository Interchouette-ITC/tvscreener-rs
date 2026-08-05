# API overview

Authoritative reference is **rustdoc** (`make doc` → [api-rust/tvscreener](../api-rust/tvscreener/index.html)).

## Crate root

| Item                         | Role                                                             |
| ---------------------------- | ---------------------------------------------------------------- |
| `ScreenerRow`                | One scan row (`symbol` + label-keyed `data`)                     |
| `Result` / `TvscreenerError` | Error type                                                       |
| Re-exports                   | `FieldDef`, `FilterOperator`, `FieldCondition`, `resolve_field`, `catalog_len`, presets, markets |

## Modules

| Module    | Role                                                     |
| --------- | -------------------------------------------------------- |
| `core`    | `Screener` builder + six typed screeners; `with_asset_screener` / `get_for_asset` |
| `field`   | `FieldDef`, defaults (`default_fields`), presets, markets                   |
| `filter`  | Operators, `Filter`, `FieldCondition`, `ExtraFilter`     |
| `util`    | URL, headers, millify, `format_value` / `format_row`     |
| `beautify`| Field-aware `format_cell` / `format_rows_table`, `CellTone` |
| `ta`      | ADX / AO / Bollinger helpers for computed recommendations |
| `resolve` | CSV token parse + `resolve_*_wires` (markets, indices, sectors, …) |
| `logging` | `env_debug_enabled`; `init_logging` (feature `cli`)      |
| `error`   | Errors (`thiserror`)                                     |
| `mcp`     | MCP tools + mcpkit server (feature `mcp`)                |
| `tui`     | Ratatui pane (feature `tui`; `tvscreener-tui` binary)    |
| `query_config` | Shared filter/sort parse for CLI / TUI / MCP        |

Default features compile the lean library only. Enable `mcp` / `tui` / `apps` for app modules.

## Binaries

| Binary           | How to run                                              | Role                                                                            |
| ---------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `tvscreener`     | `cargo install --path . --features apps` then `tvscreener …` | Default CLI (`scan`, `payload`, `presets`, `fields`, …); no subcommand → interactive prompt |
| `tvscreener-mcp` | same install, then `tvscreener-mcp`                     | Stdio MCP by default; `--http` for Streamable HTTP. See [MCP server](../README.md#mcp-server-ai-assistants). |
| `tvscreener-tui` | same install, then `tvscreener-tui …`                   | Ratatui results pane. See [TUI](../guide/tui.md).                               |
