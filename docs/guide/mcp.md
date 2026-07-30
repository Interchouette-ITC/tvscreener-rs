# MCP server (`tvscreener-mcp`)

`tvscreener-mcp` exposes screener discovery and query helpers over the
[Model Context Protocol](https://modelcontextprotocol.io/). Same field catalog
and scanner clients as the library and CLI.

## Transports

| Mode | How | Typical use |
| ---- | --- | ----------- |
| **stdio** (default) | `tvscreener-mcp` or `make run-mcp` | Local MCP clients |
| **Streamable HTTP** | `tvscreener-mcp --http` | Network clients; Docker default |

```bash
tvscreener-mcp --help
tvscreener-mcp                          # stdio
tvscreener-mcp --http                   # 0.0.0.0:8787
tvscreener-mcp --http --listen 127.0.0.1:8787

make run-mcp
cargo run --bin tvscreener-mcp
```

HTTP listen address also accepts `TVSCREENER_MCP_ADDR`.

## Docker

The image runs MCP over **HTTP on port 8787** (not stdio). See
[`docker/README.md`](../../docker/README.md).

```bash
docker run -d -p 8787:8787 interchouette/tvscreener-rs:dev
```

## Tools (summary)

- Discovery: `discover_fields`, catalog helpers (`list_markets`, `list_sectors`, …)
- Queries: `custom_query`, `search_stocks` / `search_crypto` / `search_forex`, `get_top_movers`
- Presets / payload: `list_presets`, `get_preset`, `build_payload`

Exact tool names and arguments: `tvscreener-mcp --help` and the MCP client’s tool list after connect.

## Related

- [TUI](tui.md) - interactive Builder / Results
- [Filtering](filtering.md) - operators and conditions
- [Installation](../getting-started/installation.md)
