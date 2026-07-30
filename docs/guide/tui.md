# TUI (`tvscreener-tui`)

Optional Ratatui results pane (Cargo feature `tui`). Configure the scan with CLI flags, then browse, refresh, and copy rows in the terminal.

## Build and run

```bash
cargo run --features tui --bin tvscreener-tui -- --help
make run-tui ARGS='crypto --preset crypto_price --limit 10'
make run-tui ARGS='stock --preset stock_price --index SP500 --watch'
```

Install with the feature enabled:

```bash
cargo install --path . --features tui
tvscreener-tui crypto --preset crypto_price --limit 10
```

`make lint` / clippy also run with `--features tui`.

## Keys

| Key | Action |
| --- | --- |
| `q` / Esc / Ctrl-C | Quit |
| `r` | Refresh scan (minimum **10s** between scans) |
| `a` | Toggle **watch** (opt-in auto-refresh; default interval **30s**) |
| `c` | Copy current rows as JSON (OSC 52 terminal clipboard) |
| `h` | Help pane |
| ↑ / ↓ / PgUp / PgDn | Scroll results |

## Refresh policy

- Default is **manual**: one scan at start, then `r` only.
- `--watch` starts with auto-refresh on; `a` toggles it.
- `--interval <secs>` sets the watch period (floored to **10s**).
- Library `stream()` floors at `MIN_STREAM_INTERVAL_SECS` (**1s**). That is an API floor, not the TUI default. Prefer the TUI’s 10s floor so the pane does not hammer TradingView.

## Clipboard

`c` uses **OSC 52** (no `xclip` / `wl-clipboard` dependency). Supported by many Linux terminals; tmux may need clipboard passthrough.

## Related

- Field-aware cell formatting: `format_cell` / `format_rows_table` (library; also used by CLI `scan --format table`)
- One-shot / machine output: `tvscreener scan` / `--json`
- Periodic library polls: [Streaming](streaming.md)
