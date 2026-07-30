# Installation

Rust **1.85+** (`rust-version` in `Cargo.toml`; required by `mcpkit`).

## Install the CLI

```bash
# from this repository (installs `tvscreener`, `tvscreener-mcp`, `tvscreener-tui`)
cargo install --path .
tvscreener --help
tvscreener scan crypto --limit 5
tvscreener-mcp
tvscreener-tui crypto --limit 5
```

From crates.io (when published): `cargo install tvscreener`.

From a checkout without installing: `cargo run -- …`.

## From source (develop this repository)

```bash
cargo build
cargo test
make doc            # rustdoc → docs/api-rust/
```

## As a path dependency

```toml
[dependencies]
tvscreener = { path = "../tvscreener-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Features

| Feature     | Purpose                                  |
| ----------- | ---------------------------------------- |
| _(default)_ | Library + CLI + MCP + TUI binaries       |
| `live`      | Live HTTP tests (`make test-live`)       |
| `regen`     | `tvscreener regen-fields` maintainer cmd |

```bash
cargo test --features live          # still needs TVSCREENER_LIVE=1 for e2e
cargo run --bin tvscreener-tui -- crypto --limit 5
```

## Field catalog regen

`data/fields.json` embeds the full Python field enums (thousands of columns) plus
curated defaults/presets. Regenerate from a local [deepentropy/tvscreener](https://github.com/deepentropy/tvscreener) clone (maintainer tool):

```bash
make regen-fields PYTHON_ROOT=/path/to/tvscreener
# or: cargo run --features regen --bin tvscreener -- regen-fields --python-root /path/to/tvscreener
```

Requires `--features regen` (optional maintainer tooling; not in the default CLI).
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
make docker-build-dev   # :dev tags (Hub + GHCR names)

docker pull interchouette/tvscreener-rs:dev
docker run -d -p 8787:8787 interchouette/tvscreener-rs:dev
docker run -it -p 8787:8787 interchouette/tvscreener-rs:dev
docker run --rm interchouette/tvscreener-rs:dev tvscreener --help
```

Image ships CLI, TUI, and MCP (HTTP on **8787**). See [docker/README.md](../../docker/README.md).
