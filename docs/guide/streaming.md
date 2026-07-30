# Streaming

`Screener::stream` and `stream_with_callback` poll `get` on an interval. The interval is floored by `MIN_STREAM_INTERVAL_SECS`.

Failed polls are logged with `tracing::warn` and skipped; only successful batches are collected or passed to the callback.

## Collect snapshots

```rust
use tvscreener::core::crypto::CryptoScreener;
use tvscreener::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let screener = CryptoScreener::new();
    let snapshots = screener.inner().stream(5.0, Some(3)).await;
    println!("{} polls", snapshots.len());
    Ok(())
}
```

## Callback form

```rust
screener
    .inner()
    .stream_with_callback(5.0, Some(3), |rows| {
        println!("{} rows", rows.len());
    })
    .await;
```

Prefer a single `get()` for one-shot scans. See rustdoc: [`Screener::stream`](../api-rust/tvscreener/core/struct.Screener.html#method.stream).
