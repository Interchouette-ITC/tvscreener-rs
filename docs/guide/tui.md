# TUI (`tvscreener-tui`)

Ratatui TUI for browsing screener results, editing a query in the terminal, and inspecting the request payload.

## Build and run

```bash
cargo run --features apps --bin tvscreener-tui -- --help
cargo run --features apps --bin tvscreener-tui -- crypto --preset crypto_price --limit 10
make run-tui ARGS='crypto --preset crypto_price --limit 10'
make run-tui ARGS='stock --preset stock_price --index SP500 --watch'
```

Install (ships all three binaries):

```bash
cargo install --path . --features apps
tvscreener-tui crypto --preset crypto_price --limit 10
```

Default asset at launch is the positional argument (`stock`, `crypto`, `forex`, `bond`, `futures`, `coin`). You can switch asset in **Builder** without restarting.

## Views

| Key | Pane | Purpose |
| --- | --- | --- |
| `1` / Tab | **Results** | Beautified table (same tones as CLI `scan --format table`) |
| `2` | **Builder** | Asset, preset, limit, search, sort, markets/index (stock), filters |
| `3` | **Payload** | Pretty `build_payload()` JSON (no network) |
| `4` | **Codegen** | Rust snippet + equivalent CLI lines (`tvscreener-tui` and `tvscreener scan`) |
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
| `←` / `→` | Cycle **asset** (Asset row) or **preset** (Preset row) |
| `+` / `-` | Row limit (1–500) |
| `s` | Edit name search (type, Enter save, Esc cancel) |
| `t` / `u` | Edit sort field / toggle ascending (Sort row) |
| `m` / `i` | Edit stock markets / index CSV (Stock only; row focused) |
| `f` | Add field filter: search catalog, pick field, cycle operator (`o` or `j`/`k`), type value, Enter |
| `d` | Remove selected filter |
| `j` / `k` | Move focus between builder rows, or select filter row |
| `Enter` | Apply config and run scan |

Switching asset resets preset to defaults, clears filters, and clears markets/index when leaving Stock. Status shows `asset changed — Enter to scan`.

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

Launch flags mirror `tvscreener scan` / `payload` query options:

```bash
tvscreener-tui crypto --preset crypto_price --limit 25 --search BTC --watch --interval 30
tvscreener-tui stock --index SP500 --markets AMERICA --limit 20
tvscreener-tui stock --filter close:greater:100 --sort-by volume --ascending
tvscreener-tui stock --filters '[{"field":"close","op":">","value":100}]'
```

| Flag | Meaning |
| --- | --- |
| `--filter FIELD:OP:VALUE` | Repeatable filter token (e.g. `close:greater:100`) |
| `--filters '<json>'` | JSON array/object of `{field, op, value}` |
| `--sort-by <field>` | Sort column (const, technical, or label) |
| `--ascending` | Sort ascending (default descending) |
| `--markets`, `--index` | Stock scope CSV (const or wire) |

Codegen emits equivalent `tvscreener scan …` lines with the same flags.

## Related

- Field-aware cell formatting: `format_cell` / `format_rows_table` (library; also used by CLI `scan --format table`)
- One-shot / machine output: `tvscreener scan` / `payload` / `--json`
- Periodic library polls: [Streaming](streaming.md)
