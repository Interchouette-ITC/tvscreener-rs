# Generated field / enum modules

Produced by `tvscreener regen-fields` (or `make regen-fields PYTHON_ROOT=…`) from the
Python field / `__init__` enums into `data/fields.json` and these Rust modules:

- `index_symbol.rs`
- `sector.rs` / `country.rs` / `industry.rs` / `exchange.rs`
- `rating.rs`

Do not edit by hand - run `make regen-fields PYTHON_ROOT=/path/to/tvscreener`.
