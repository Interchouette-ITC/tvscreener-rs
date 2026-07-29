# Filtering

Filters become objects in the scanner JSON `filter` array (`left`, `operation`, `right`).

## Operators

`FilterOperator` maps to TradingView wire strings (same set as Python `FilterOperator`):

| Variant                                 | Wire `operation`                              |
| --------------------------------------- | --------------------------------------------- |
| `Below` / `BelowOrEqual`                | `less` / `eless`                              |
| `Above` / `AboveOrEqual`                | `greater` / `egreater`                        |
| `Equal` / `NotEqual`                    | `equal` / `nequal`                            |
| `InRange` / `NotInRange`                | `in_range` / `not_in_range`                   |
| `Match`                                 | `match`                                       |
| `Crosses` / `CrossesUp` / `CrossesDown` | `crosses` / `crosses_above` / `crosses_below` |

```rust
use tvscreener::filter::{FieldCondition, FilterOperator};
use serde_json::json;

let condition = FieldCondition::new("close", FilterOperator::Above, json!(100));
```

## Attach to a screener

```rust
screener.where_condition(condition);
// or low-level:
screener.add_filter(Filter::from(condition));
```

## Merge rule

Filters that share the same `left` key are **merged**: values accumulate and the operator may promote to `in_range`. This matches the Python client.

## Search helper

Asset screeners expose `search(...)` for name/match-style filters (e.g. crypto `search("BTC")`).

## Extra filters

`ExtraFilter` covers scanner extras beyond simple field conditions (see rustdoc).
