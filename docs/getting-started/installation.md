# Installation

## From source (this repository)

```bash
cargo build
cargo test
make doc            # rustdoc → docs/api-rust/
```

Rust **1.85+** (`rust-version` in `Cargo.toml`; required by `mcpkit`).

## As a path dependency

```toml
[dependencies]
tvscreener = { path = "../tvscreener-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Features

| Feature     | Purpose                                  |
| ----------- | ---------------------------------------- |
| _(default)_ | Library + `tvscreener` CLI               |
| `live`      | Live HTTP tests (`make test-live`)       |
| `mcp`       | `tvscreener-mcp` stdio server (`mcpkit`) |

```bash
make run ARGS='--help'
make run ARGS='payload stock --limit 2'
cargo test --features live          # still needs TVSCREENER_LIVE=1 for e2e
make run-mcp                        # or: cargo run --features mcp --bin tvscreener-mcp
```

## Field catalog regen

`data/fields.json` embeds the full Python field enums (thousands of columns) plus
curated defaults/presets. Regenerate from a local [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener) clone (maintainer tool):

```bash
make regen-fields PYTHON_ROOT=/path/to/tvscreener
# or: cargo run --bin tvscreener -- regen-fields --python-root /path/to/tvscreener
```

Use `search_fields` / `list_fields` / `catalog_len` for MCP-oriented discovery;
typed screeners still select **defaults** only unless you `select` more.

## Supply chain (local)

```bash
cargo install cargo-audit cargo-deny   # once
make audit                             # rustsec advisories
make deny                              # licenses / bans (deny.toml)
```

Run locally before releases (`make audit` / `make deny`).

## Docker

```bash
make docker-build   # gregoshop/tvscreener-rs:latest
```

See [docker/README.md](../../docker/README.md).
