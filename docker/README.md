# Docker image (tvscreener-mcp)

Size-optimized multi-stage build → `debian:bookworm-slim` runtime (ca-certificates only, non-root).

| Item       | Value                                                 |
| ---------- | ----------------------------------------------------- |
| Image      | `gregoshop/tvscreener-rs`                             |
| Entrypoint | `/usr/local/bin/tvscreener-mcp` (stdio MCP)           |
| TLS        | rustls + `ca-certificates` (no OpenSSL package)       |
| Tags       | `:latest` and `:$(APP_VERSION)` from `Cargo.toml`     |
| Compose    | `docker-compose.prod.yml` / `docker-compose.test.yml` |

## Build / push / run

From repo root:

```bash
make docker-build          # gregoshop/tvscreener-rs:latest + :$(APP_VERSION)
make docker-push           # Docker Hub (login first)
make docker-run            # compose prod up -d
make docker-run-test       # compose test up (foreground)
make docker-stop           # compose prod stop
make docker-build-no-cache
make docker-inspect
```

Overrides: `REGISTRY=gregoshop TAG=latest APP_VERSION=1.0.0`

## Entrypoint behavior

The image runs `tvscreener-mcp` on stdio (`stdin_open` / `tty` in compose).
Build with `--features mcp`. Tools: discover*fields, custom_query, search*\*, presets, …

## Notes

- `data/fields.json` is compiled in via `include_str!` - not copied into the runtime layer.
- Binary is `strip`’d; release profile uses `lto`, `codegen-units=1`, `opt-level=s`, `panic=abort`.
- Builder caches crate deps in a dummy layer before copying real `src/` / `data/`.
- Runtime stays slim: only `ca-certificates` + non-root user (no shell tools required at runtime).
