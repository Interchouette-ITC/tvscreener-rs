# tvscreener-rs

Unofficial **Rust** TradingView Screener client image: **CLI**, **TUI**, and **MCP** (Streamable HTTP).

Port of [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener). **Not affiliated with TradingView.** Use is subject to [TradingView’s terms](https://www.tradingview.com/policies/).

Source: [Interchouette-ITC/tvscreener-rs](https://github.com/Interchouette-ITC/tvscreener-rs)

## What’s inside

| Binary | Role |
| --- | --- |
| `tvscreener` | CLI (interactive prompt or one-shot `scan` / `payload` / catalog) |
| `tvscreener-tui` | Ratatui TUI (Results, Builder, Payload, Codegen) |
| `tvscreener-mcp` | MCP server (Streamable HTTP in Docker; stdio when run locally) |

Size-optimized multi-stage build → `debian:bookworm-slim` (ca-certificates, non-root). TLS via rustls (no OpenSSL package).

## Quick start

```bash
docker pull interchouette/tvscreener-rs:dev

# MCP alone (Streamable HTTP on :8787 → /mcp)
docker run -d -p 8787:8787 interchouette/tvscreener-rs:dev

# TUI + MCP (default when attached with a TTY)
docker run -it -p 8787:8787 interchouette/tvscreener-rs:dev

# Interactive CLI + MCP
docker run -it -p 8787:8787 interchouette/tvscreener-rs:dev tvscreener

# One-shot CLI (print and exit; MCP not started)
docker run --rm interchouette/tvscreener-rs:dev tvscreener --help
docker run --rm interchouette/tvscreener-rs:dev tvscreener payload crypto --limit 2
docker run --rm interchouette/tvscreener-rs:dev tvscreener scan crypto --limit 5
```

## Environment

| Env | Default | Meaning |
| --- | --- | --- |
| `ENABLE_MCP` | `1` | Start MCP HTTP beside long-lived TUI / interactive CLI; ignored for one-shot CLI |
| `TVSCREENER_MCP_ADDR` | `0.0.0.0:8787` | Bind address for MCP HTTP |

AI clients that support Streamable HTTP can use `http://localhost:8787/mcp` when the port is published.

## Tags

| Tag | Meaning |
| --- | --- |
| `:dev` | Latest development image (manual / on-demand push) |
| `:X.Y.Z` | Release matching `Cargo.toml` / GitHub Release `vX.Y.Z` |
| `:latest` | Moves with each release |

## Features (library + tools)

- Query **Stock**, **Crypto**, **Forex**, **Bond**, **Futures**, and **Coin** screeners
- **13,000+ fields** embedded in the catalog
- Fluent builders, field presets, async HTTP (`reqwest` + rustls)
- CLI table/json output, TUI builder + codegen, MCP for AI assistants

## Docs

- Repo docs: [github.com/Interchouette-ITC/tvscreener-rs](https://github.com/Interchouette-ITC/tvscreener-rs)
- Docker details: [`docker/README.md`](https://github.com/Interchouette-ITC/tvscreener-rs/blob/dev/docker/README.md)
- Website: [interchouette.net](https://interchouette.net/)

## License / disclaimer

See the repository for license terms. This project is **not** affiliated with TradingView.
