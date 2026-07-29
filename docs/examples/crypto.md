# Example: crypto

Runnable example in the repo:

```bash
cargo run --example example_crypto
```

Requires network access to `scanner.tradingview.com`.

## What it demonstrates

1. **`crypto_price` preset** - name, price, change %
2. **`crypto_volume` + `search("BTC")`**
3. **`crypto_performance` preset** - inspect returned labels
4. **Technical filter** - RSI below a threshold via `where_condition`
5. Terminal printing with `util::format_value`

Offline companion: `cargo run --example manual_test` (util / presets / display, no HTTP).

Coverage map: [MANUAL_TEST_PLAN.md](../MANUAL_TEST_PLAN.md).
