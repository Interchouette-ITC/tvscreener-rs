# Docker image (tvscreener-mcp)

Size-optimized multi-stage build → `debian:bookworm-slim` runtime (ca-certificates only, non-root).

| Item       | Value                                                       |
| ---------- | ----------------------------------------------------------- |
| Entrypoint | `/usr/local/bin/tvscreener-mcp` (stdio MCP)                 |
| TLS        | rustls + `ca-certificates` (no OpenSSL package)             |
| Compose    | `docker-compose.prod.yml` / `docker-compose.test.yml`       |

## Registries

| Registry | Image |
| -------- | ----- |
| Docker Hub | [`gregoshop/tvscreener-rs`](https://hub.docker.com/r/gregoshop/tvscreener-rs) |
| GHCR (personal) | `ghcr.io/groussac/tvscreener-rs` ([packages](https://github.com/gRoussac?tab=packages)) |
| GHCR (org) | `ghcr.io/interchouette-itc/tvscreener-rs` ([packages](https://github.com/orgs/Interchouette-ITC/packages)) |

## Tags

| Tag | Who pushes | When |
| --- | ---------- | ---- |
| `:dev` | You (local `make`) or Actions `workflow_dispatch` | On demand |
| `:X.Y.Z` | GitHub Actions on **Release** | Tag must be `vX.Y.Z` matching `Cargo.toml` |
| `:latest` | Same release workflow | Moves with each release |

## Local build / push `:dev`

```bash
make docker-build-dev
make docker-push-dev          # docker login (Hub); optional GHCR login
# CI=1 skips interactive login (used by Actions)
```

## Release images (GitHub-owned)

1. `make version-show` / `make version-bump-patch` (etc.)
2. Create a GitHub Release with tag `v$(APP_VERSION)` (must match `Cargo.toml`)
3. Workflow `Release - Build and Push Versioned Images` pushes `:X.Y.Z` and `:latest` to Hub + both GHCR

## Other make targets

```bash
make docker-build             # :latest + :$(APP_VERSION) on Hub name
make docker-run               # compose prod up -d
make docker-run-test          # compose test up (foreground)
make docker-stop
make docker-build-no-cache
make docker-inspect
make version-show
```

## Secrets (Actions — fill later)

| Secret | Use |
| ------ | --- |
| `DOCKER_USERNAME` / `DOCKER_PASSWORD` | Docker Hub |
| `GHCR_USERNAME` / `GHCR_PAT` | `ghcr.io` (gRoussac + Interchouette-ITC packages) |

## Entrypoint behavior

The image runs `tvscreener-mcp` on stdio (`stdin_open` / `tty` in compose).
Build with `--features mcp`.

## Notes

- `data/fields.json` is compiled in via `include_str!` - not copied into the runtime layer.
- Binary is `strip`’d; release profile uses `lto`, `codegen-units=1`, `opt-level=s`, `panic=abort`.
- Builder caches crate deps in a dummy layer before copying real `src/` / `data/`.
- Runtime stays slim: only `ca-certificates` + non-root user.
