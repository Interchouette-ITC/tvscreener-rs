# Manual test plan (tvscreener-rs)

How to run the test suite and related examples.

## Prerequisites

- Rust **1.85+**
- Live tests need network and `TVSCREENER_LIVE=1`

## Commands

| Goal                | Action                |
| ------------------- | --------------------- |
| Tests               | `make test`           |
| Lint                | `make lint`           |
| Verify              | `make verify`         |
| Manual example      | `make example-manual` |
| Live crypto         | `make example-crypto` |
| Live e2e            | `make test-live`      |
| CLI                 | `tvscreener …`        |
| MCP server          | `tvscreener-mcp`      |
| Advisories          | `make audit`          |
| Licenses            | `make deny`           |
| Regen field catalog | `make regen-fields`   |

`make test` does not call the scanner. `make test-live` sets `TVSCREENER_LIVE=1` and `--features live`.

## What covers what

| Area                                             | Tests / examples                                    |
| ------------------------------------------------ | --------------------------------------------------- |
| Util (`get_url`, `millify`, headers)             | `src/util.rs`, `examples/manual_test.rs`            |
| Fields / search / defaults / presets             | `offline_coverage`, `src/field`                     |
| Filters                                          | `offline_coverage`, `src/core`, `src/filter`        |
| Payloads (`select`, range, sort, index, markets) | `offline_coverage`, `offline_payload_goldens`       |
| Typed screeners `get()` / stream                 | `e2e_live`                                          |
| Display formatting                               | `src/util`, `examples/manual_test.rs`               |
| Errors                                           | `offline_coverage`                                  |
| CLI (no scanner)                                 | `tests/cli.rs`                                      |
| CLI (live scanner: `scan`, preset, index, error) | `tests/e2e/cli_live.rs` (`--features live`)         |

| MCP tools | `cargo test` (includes `src/mcp`) |

## Checklist

- [ ] `cargo test`
- [ ] `make lint` / `make verify`
- [ ] `cargo run --example manual_test`
- [ ] `TVSCREENER_LIVE=1 cargo test --features live --test e2e_live -- --test-threads=1`
- [ ] `TVSCREENER_LIVE=1 cargo test --features live --test cli_live -- --test-threads=1`
- [ ] `tvscreener scan crypto --limit 3`
- [ ] `tvscreener-mcp`

## Notes

- Bond/futures/coin Python `DEFAULT_*_FIELDS` lists are empty upstream; this crate uses curated defaults / presets.
- `select_all` selects curated catalog defaults, not the full Python enum.
- Terminal `format_*` / `format_cell` helpers are covered; HTML table export is out of scope.
- Optional TUI: `cargo test --features tui --lib tui::`; live smoke via `make run-tui ARGS='crypto --limit 3'`.
