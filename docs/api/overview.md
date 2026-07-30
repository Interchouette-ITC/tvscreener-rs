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
| `resolve` | CSV token parse + `resolve_*_wires` (markets, indices, sectors, …) |
| `logging` | `env_debug_enabled`; `init_logging`                      |
| `error`   | Errors (`thiserror`)                                     |
| `mcp`     | MCP tools + mcpkit stdio server (`feature = "mcp"` only) |

## Binaries

| Binary           | Make target    | Role                                                                            |
| ---------------- | -------------- | ------------------------------------------------------------------------------- |
| `tvscreener`     | `make run`     | Default CLI (`scan`, `payload`, `presets`, `fields`, …)                         |
| `tvscreener-mcp` | `make run-mcp` | Stdio MCP (`--features mcp`). See [MCP server](../../README.md#mcp-server-ai-assistants). |
