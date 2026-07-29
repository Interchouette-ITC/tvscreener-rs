# Sorting and range

## Range

```rust
screener.set_range(0, 50); // window on the scanner result set
```

Defaults follow the Python client (`DEFAULT_MIN_RANGE` / `DEFAULT_MAX_RANGE`, typically `0..150`).

## Sort

```rust
screener.sort_by("volume|24h|USD", false); // field_name, ascending?
```

Typed screeners set a sensible default sort (e.g. crypto by 24h USD volume descending).

## Payload inspection

```rust
let payload = screener.build_payload()?;
println!("{}", serde_json::to_string_pretty(&payload)?);
```

Enable request logging with `set_print_request(true)` on the shared `Screener` when debugging.
