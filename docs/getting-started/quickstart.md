# Quick start

Get a first screener response in a few minutes.

## CLI (no code)

```bash
cargo install --path .
tvscreener --help
tvscreener scan crypto --limit 5
tvscreener payload crypto --limit 2     # print request JSON only
tvscreener-mcp
```

## Create a screener and fetch rows

```rust
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let screener = CryptoScreener::new();
    let rows = screener.get().await?; // Vec<ScreenerRow>
    println!("{} rows", rows.len());
    Ok(())
}
```

`get()` returns **`Vec<ScreenerRow>`**. Each row has:

- `symbol` - e.g. `"BINANCE:BTCUSDT"`
- `data` - map of **column label** → JSON value (not technical field names)

## Filter

```rust
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::field::crypto_rsi_14;
use tvscreener::filter::{FieldCondition, FilterOperator};
use tvscreener::Result;
use serde_json::json;

fn apply_rsi_filter(screener: &mut CryptoScreener) -> Result<()> {
    screener.where_condition(FieldCondition::new(
        crypto_rsi_14().field_name,
        FilterOperator::Below,
        json!(35),
    ))?;
    Ok(())
}
```

`where_condition` returns `Result`; propagate with `?`.

## Select columns

```rust
use tvscreener::field::{crypto_price, get_preset};

screener.select(get_preset("crypto_price")?);
// or
screener.select([crypto_price()]);
```

## Limit the window

```rust
screener.set_range(0, 10); // [from, to) style range on the scanner
```

## Format for the terminal

```rust
use tvscreener::util::format_row;

for row in &rows {
    println!("{}", format_row(row, Some(&["Name", "Price", "Change %"])));
}
```

## Next

- [Filtering](../guide/filtering.md)
- [Selecting fields](../guide/selecting-fields.md)
- [Crypto example](../examples/crypto.md)
