# Docker image (CLI, TUI, MCP)

Size-optimized multi-stage build → `gcr.io/distroless/cc-debian13:nonroot` (Debian 13 / trixie family; no apt, perl, or shell).

| Item | Value |
| --- | --- |
| Binaries | `tvscreener`, `tvscreener-mcp`, `tvscreener-tui`, `tvscreener-entrypoint` |
| Entrypoint | `/usr/local/bin/tvscreener-entrypoint` |
| Builder | `rust:slim-trixie` |
| MCP (Docker) | Streamable HTTP on **6790** (`/mcp`) |
| MCP (local) | `tvscreener-mcp` stdio by default |
| TLS | rustls + CA bundle copied into the image (no OpenSSL package) |
| Compose | `docker-compose.prod.yml` / `docker-compose.test.yml` |

## Where to pull images (public)

| Registry | Image | Packages page |
| --- | --- | --- |
| Docker Hub | `interchouette/tvscreener-rs` | [hub.docker.com/r/interchouette/tvscreener-rs](https://hub.docker.com/r/interchouette/tvscreener-rs) |
| Docker Hub | `gregoshop/tvscreener-rs` (legacy mirror) | [hub.docker.com/r/gregoshop/tvscreener-rs](https://hub.docker.com/r/gregoshop/tvscreener-rs) |
| GHCR | `ghcr.io/interchouette/tvscreener-rs` | [Interchouette packages](https://github.com/Interchouette?tab=packages) |
| GHCR | `ghcr.io/interchouette-itc/tvscreener-rs` | [Interchouette-ITC packages](https://github.com/orgs/Interchouette-ITC/packages) |

Public pulls: Docker Hub `interchouette/tvscreener-rs` (and legacy `gregoshop/tvscreener-rs`), plus GHCR `ghcr.io/interchouette/tvscreener-rs` and `ghcr.io/interchouette-itc/tvscreener-rs`.

```bash
docker pull interchouette/tvscreener-rs:dev
docker pull ghcr.io/interchouette/tvscreener-rs:dev
docker pull ghcr.io/interchouette-itc/tvscreener-rs:dev
```

## Run modes

```bash
IMAGE=interchouette/tvscreener-rs:dev

# MCP alone (Streamable HTTP on :6790)
docker run -d -p 6790:6790 "$IMAGE"

# TUI + MCP (default when attached with a TTY)
docker run -it -p 6790:6790 "$IMAGE"

# TUI only
docker run -it -p 6790:6790 -e ENABLE_MCP=0 "$IMAGE"

# Interactive CLI + MCP (no subcommand → prompt until quit / exit)
docker run -it -p 6790:6790 "$IMAGE" tvscreener

# Interactive CLI only
docker run -it --rm -e ENABLE_MCP=0 "$IMAGE" tvscreener

# One-shot CLI (print and exit; MCP not started)
docker run --rm "$IMAGE" tvscreener --help
docker run --rm "$IMAGE" tvscreener payload crypto --limit 2
```

| Env | Default | Meaning |
| --- | --- | --- |
| `ENABLE_MCP` | `1` | Start MCP HTTP beside long-lived TUI / interactive CLI; ignored for one-shot CLI |
| `TVSCREENER_MCP_ADDR` | `0.0.0.0:6790` | Bind address for MCP HTTP |

AI clients that support Streamable HTTP can use `http://localhost:6790/mcp` when the port is published. Local Cursor setups can keep spawning `tvscreener-mcp` on **stdio**.

## Tags

| Tag | Who pushes | When |
| --- | --- | --- |
| `:dev` | Local `make` or Actions `workflow_dispatch` (“CI/CD Image dev”) | On demand |
| `:X.Y.Z` | GitHub Actions on **Release** | Tag `vX.Y.Z` must match `Cargo.toml` |
| `:latest` | Same release workflow | Moves with each release |

## Release images and binaries

| Piece | Status |
| --- | --- |
| CI (audit, lint, test, artifacts, rustdoc pages) | Live on `dev` |
| `:dev` image push (Hub + GHCR) | Live |
| Versioned release images `:X.Y.Z` + `:latest` | Cut with GitHub Release tag `vX.Y.Z` (= `Cargo.toml`) |
| Release binaries (`tvscreener`, `tvscreener-mcp`, `tvscreener-tui`) | Attached on that Release |

First product release: tag **`v1.0.0`** when `Cargo.toml` is `1.0.0` (see root [`CHANGELOG.md`](../CHANGELOG.md)).

To cut a release:

1. `make version-show` (or bump with `make version-bump-patch` etc.)
2. Merge version / changelog to `dev` if needed
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
| --- | --- |
| `DOCKER_USERNAME` / `DOCKER_PASSWORD` | Docker Hub (`interchouette/...` and legacy `gregoshop/...`) |
| `GHCR_USERNAME` / `GHCR_PAT` | Extra personal GHCR mirror (optional for docs) |
| `GHCR_USERNAME_ITC` / `GHCR_PAT_ITC` | `ghcr.io/interchouette/...` and `ghcr.io/interchouette-itc/...` |

Hub **Overview** text is maintained in [`DOCKERHUB.md`](DOCKERHUB.md) and synced with `make docker-hub-description` (also after Hub image pushes).

## Notes

- `data/fields.json` is compiled in via `include_str!`.
- Binaries are stripped; release profile uses `lto`, `codegen-units=1`, `opt-level=s`, `panic=abort`.
- Runtime: distroless `cc-debian13` + non-root; entrypoint is a Rust binary (no `/bin/sh`).
- Default TUI launch (no args + TTY): `tvscreener-tui crypto --limit 25`.
