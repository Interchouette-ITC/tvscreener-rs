# Selecting fields

Columns in the POST body come from `FieldDef` values. Response maps are keyed by **`FieldDef.label`**, not `field_name`.

## FieldDef

Each field carries:

- `label` - human / response key (e.g. `"Price"`)
- `field_name` - technical / wire column (e.g. `"close"`)
- optional format / interval / historical metadata

Helpers such as `crypto_price()`, `crypto_rsi_14()`, and `default_crypto_fields()` live in `tvscreener::field`.

## Presets

```rust
use tvscreener::field::{get_preset, list_presets};

let names = list_presets();
let fields = get_preset("crypto_price")?;
screener.select(fields);
```

## select / select_all

```rust
screener.select(fields);           // replace column set
screener.select_all()?;            // needs set_all_fields_fn (typed screeners set this)
```

Typed constructors (`CryptoScreener::new`, …) register defaults and an all-fields callback.
