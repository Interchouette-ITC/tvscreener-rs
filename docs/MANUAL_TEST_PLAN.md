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
| CLI                 | `make run ARGS='…'`   |
| MCP server          | `make run-mcp`        |
| Advisories          | `make audit`          |
| Licenses            | `make deny`           |
| Regen field catalog | `make regen-fields`   |

`make test` does not call the scanner. `make test-live` sets `TVSCREENER_LIVE=1` and `--features live`.

## What covers what

| Area                                             | Tests / examples                                                      |
| ------------------------------------------------ | --------------------------------------------------------------------- |
| Util (`get_url`, `millify`, headers)             | `tests/offline_coverage.rs`, `src/util.rs`, `examples/manual_test.rs` |
| Fields / search / defaults / presets             | `offline_coverage`, `src/field`                                       |
| Filters                                          | `offline_coverage`, `src/core`, `src/filter`                          |
| Payloads (`select`, range, sort, index, markets) | `offline_coverage`, `offline_payload_goldens`                         |
| Typed screeners `get()` / stream                 | `e2e_live`                                                            |
| Display formatting                               | `offline_coverage`, `src/util`, `manual_test`                         |
| Errors                                           | `offline_coverage`                                                    |
| CLI (no scanner)                                 | `tests/cli.rs`                                                        |
| CLI (live scanner: `scan`, preset, index, error) | `tests/e2e/cli_live.rs` (`--features live`)                           |

| MCP tools | `cargo test --features mcp` |

## Checklist

- [ ] `cargo test`
- [ ] `make lint` / `make verify`
- [ ] `cargo run --example manual_test`
- [ ] `TVSCREENER_LIVE=1 cargo test --features live --test e2e_live -- --test-threads=1`
- [ ] `TVSCREENER_LIVE=1 cargo test --features live --test cli_live -- --test-threads=1`
- [ ] `make run ARGS='scan crypto --limit 3'`
- [ ] `make run-mcp`

## Notes

- Bond/futures/coin Python `DEFAULT_*_FIELDS` lists are empty upstream; this crate uses curated defaults / presets.
- `select_all` selects curated catalog defaults, not the full Python enum.
- Terminal `format_*` helpers are covered; Jupyter-style table styling is out of scope.
