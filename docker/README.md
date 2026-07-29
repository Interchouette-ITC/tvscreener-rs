# Docker image (tvscreener-mcp)

Size-optimized multi-stage build → `debian:bookworm-slim` runtime (ca-certificates only, non-root).

| Item       | Value                                                       |
| ---------- | ----------------------------------------------------------- |
| Entrypoint | `/usr/local/bin/tvscreener-mcp` (stdio MCP)                 |
| TLS        | rustls + `ca-certificates` (no OpenSSL package)             |
| Compose    | `docker-compose.prod.yml` / `docker-compose.test.yml`       |

## Where to pull images (public)

| Registry | Image | Packages page |
| -------- | ----- | ------------- |
| Docker Hub | `gregoshop/tvscreener-rs` | [hub.docker.com/r/gregoshop/tvscreener-rs](https://hub.docker.com/r/gregoshop/tvscreener-rs) |
| GHCR | `ghcr.io/interchouette/tvscreener-rs` | [Interchouette packages](https://github.com/Interchouette?tab=packages) |
| GHCR | `ghcr.io/interchouette-itc/tvscreener-rs` | [Interchouette-ITC packages](https://github.com/orgs/Interchouette-ITC/packages) |

```bash
docker pull gregoshop/tvscreener-rs:dev
docker pull ghcr.io/interchouette/tvscreener-rs:dev
docker pull ghcr.io/interchouette-itc/tvscreener-rs:dev
```

GHCR packages are **private by default**. After the first push, set each package to **Public** once in Package settings → Change visibility (UI only; no API). Docker Hub `gregoshop/tvscreener-rs` is public.

## Tags

| Tag | Who pushes | When |
| --- | ---------- | ---- |
| `:dev` | Local `make` or Actions `workflow_dispatch` (“CI/CD Image dev”) | On demand |
| `:X.Y.Z` | GitHub Actions on **Release** | Tag `vX.Y.Z` must match `Cargo.toml` |
| `:latest` | Same release workflow | Moves with each release |

## Release status

| Piece | Status |
| ----- | ------ |
| CI (audit, lint, test, artifacts, rustdoc pages) | Live on `dev` |
| Manual `:dev` image push (Hub + GHCR) | Live; tested |
| Versioned release images `:X.Y.Z` + `:latest` | Ready when you cut a GitHub Release |
| Release binaries (`tvscreener`, `tvscreener-mcp`) | Attached on that same Release |

To cut a release:

1. `make version-show` (or bump with `make version-bump-patch` etc.)
2. Merge version bump to `dev` if needed
3. Create a GitHub Release with tag **`v$(APP_VERSION)`** (must equal `Cargo.toml`)
4. Workflow pushes Hub + GHCR tags and attaches Linux binaries

## Local build / push `:dev`

```bash
make docker-build-dev
make docker-push-dev
```

## Other make targets

```bash
make docker-build
make docker-run
make docker-run-test
make docker-stop
make docker-build-no-cache
make docker-inspect
make version-show
```

## Secrets (repo secrets)

| Secret | Use |
| ------ | --- |
| `DOCKER_USERNAME` / `DOCKER_PASSWORD` | Docker Hub |
| `GHCR_USERNAME` / `GHCR_PAT` | Extra personal GHCR mirror (optional for docs) |
| `GHCR_USERNAME_ITC` / `GHCR_PAT_ITC` | `ghcr.io/interchouette/...` and `ghcr.io/interchouette-itc/...` |

## Entrypoint behavior

The image runs `tvscreener-mcp` on stdio (`stdin_open` / `tty` in compose).
Build with `--features mcp`.

## Notes

- `data/fields.json` is compiled in via `include_str!`.
- Binary is stripped; release profile uses `lto`, `codegen-units=1`, `opt-level=s`, `panic=abort`.
- Runtime: `ca-certificates` + non-root user.
