# tvscreener-rs

Unofficial **Rust** port of [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener) - TradingView Screener HTTP client.

**Not affiliated with TradingView.** Use is subject to [TradingView’s terms](https://www.tradingview.com/policies/).

## Documentation

Full project docs, guides, and the manual test plan live under **[`docs/`](docs/README.md)**:

- [docs/README.md](docs/README.md) - overview, build, Docker, module map
- [docs/getting-started/](docs/getting-started/quickstart.md) - install & quick start
- [docs/MANUAL_TEST_PLAN.md](docs/MANUAL_TEST_PLAN.md) - how to run tests
- [`CHANGELOG.md`](CHANGELOG.md) - semver notes
- `make doc` → rustdoc at [docs/api-rust/tvscreener/](docs/api-rust/tvscreener/index.html)

**Crate layout:** `[lib]` + default CLI `tvscreener` + optional `tvscreener-mcp` (`--features mcp`).

## Quick start

```bash
make test          # default suite
make lint
make verify        # format-check + clippy + tests
make doc           # API HTML under docs/api-rust/
make run ARGS='--help'   # or just: make run
make run ARGS='payload crypto --limit 2'
make run ARGS='scan crypto --limit 5'   # live HTTP
make run-mcp       # stdio MCP server (blocks; --features mcp)
make docker-build  # gregoshop/tvscreener-rs
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

## License

[Apache-2.0](LICENSE). Cite [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener) for the original design.
