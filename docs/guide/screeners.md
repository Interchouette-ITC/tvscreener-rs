# Screeners

Six typed wrappers around [`Screener`](../api-rust/tvscreener/core/struct.Screener.html), each setting subtype URL, default fields, and sort/misc defaults.

| Type    | Module                      | Scanner subtype   |
| ------- | --------------------------- | ----------------- |
| Stock   | `tvscreener::core::stock`   | `global` (stocks) |
| Crypto  | `tvscreener::core::crypto`  | `crypto`          |
| Forex   | `tvscreener::core::forex`   | `forex`           |
| Bond    | `tvscreener::core::bond`    | `bond`            |
| Futures | `tvscreener::core::futures` | `futures`         |
| Coin    | `tvscreener::core::coin`    | `coin`            |

## Pattern

```rust
use tvscreener::core::stock::StockScreener;
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut screener = StockScreener::new();
    screener.set_range(0, 25);
    let rows = screener.get().await?;
    Ok(())
}
```

Use `inner_mut()` for advanced builder methods not re-exported on the wrapper.

## Low-level builder

```rust
use tvscreener::core::Screener;

let mut screener = Screener::new("crypto");
screener.set_range(0, 10);
let payload = screener.build_payload()?;
```
