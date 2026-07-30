# TUI (`tvscreener-tui`)

Optional Ratatui TUI (Cargo feature `tui`). Configure the scan at launch, edit preset/limit/search/filters in **Builder**, inspect **Payload** JSON and **Codegen**, then browse results in the terminal.

## Build and run

```bash
cargo run --features tui --bin tvscreener-tui -- --help
cargo run --features tui --bin tvscreener-tui -- crypto --preset crypto_price --limit 10
make run-tui ARGS='crypto --preset crypto_price --limit 10'
make run-tui ARGS='stock --preset stock_price --index SP500 --watch'
```

Install with the feature enabled:

```bash
cargo install --path . --features tui
tvscreener-tui crypto --preset crypto_price --limit 10
```

`make lint`, `make check`, and `make test` also run with `--features tui`.

Asset class is chosen at launch (`stock`, `crypto`, `forex`, `bond`, `futures`, `coin`).

## Views

| Key | Pane | Purpose |
| --- | --- | --- |
| `1` / Tab | **Results** | Beautified table (same tones as CLI `scan --format table`) |
| `2` | **Builder** | Preset, limit, search, filters |
| `3` | **Payload** | Pretty `build_payload()` JSON (no network) |
| `4` | **Codegen** | Rust snippet + equivalent CLI line |
| `h` | **Help** | Key reference |

`Tab` cycles Results → Builder → Payload → Codegen.

## Global keys

| Key | Action |
| --- | --- |
| `q` / Esc / Ctrl-C | Quit |
| `r` | Refresh scan (minimum **10s** between scans) |
| `a` | Toggle **watch** (opt-in auto-refresh; default interval **30s**) |
| `c` | Copy active pane (OSC 52; see below) |
| ↑ / ↓ / PgUp / PgDn | Scroll active pane |

## Builder

| Key | Action |
| --- | --- |
| `←` / `→` | Previous / next preset (when not typing) |
| `+` / `-` | Row limit (1–500) |
| `s` | Edit name search (type, Enter save, Esc cancel) |
| `f` | Add field filter: search catalog, pick field, cycle operator (`o` or `j`/`k`), type value, Enter |
| `d` | Remove selected filter |
| `j` / `k` | Move focus between builder rows, or select filter row |
| `Enter` | Apply config and run scan |

Stock `--markets` / `--index` from the CLI are shown read-only in Builder (set them on the command line).

## Refresh policy

- Default is **manual**: one scan at start, then `r` only.
- `--watch` starts with auto-refresh on; `a` toggles it.
- `--interval <secs>` sets the watch period (floored to **10s**).
- Library `stream()` floors at `MIN_STREAM_INTERVAL_SECS` (**1s**). That is an API floor, not the TUI default.

## Clipboard

`c` uses **OSC 52** (no `xclip` / `wl-clipboard` dependency):

- **Results** — rows as JSON
- **Payload** — request JSON
- **Codegen** — Rust + CLI text

Supported by many Linux terminals; tmux may need clipboard passthrough.

## CLI flags

```bash
tvscreener-tui crypto --preset crypto_price --limit 25 --search BTC --watch --interval 30
tvscreener-tui stock --index SP500 --markets AMERICA --limit 20
```

Field filters added in Builder are not yet exposed as CLI flags; use the TUI or Rust API (`where_condition`).

## Related

- Field-aware cell formatting: `format_cell` / `format_rows_table` (library; also used by CLI `scan --format table`)
- One-shot / machine output: `tvscreener scan` / `--json`
- Periodic library polls: [Streaming](streaming.md)
