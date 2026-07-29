# tvscreener-rs - developer targets (`make help`)

# Cursor agent shells inject CARGO_TARGET_DIR / PLAYWRIGHT_BROWSERS_PATH into
# cursor-sandbox-cache. Never build there; use the repo `target/` (and defaults).
unexport CARGO_TARGET_DIR
unexport PLAYWRIGHT_BROWSERS_PATH

CARGO_BIN ?= cargo
# Belt-and-suspenders: strip even if a parent re-exports into the recipe shell.
CARGO = env -u CARGO_TARGET_DIR -u PLAYWRIGHT_BROWSERS_PATH $(CARGO_BIN)
CARGO_FLAGS ?=
NIGHTLY_FLAGS ?=
TVSCREENER_LIVE ?= 1

REGISTRY ?= gregoshop
TAG ?= latest
APP_IMAGE = $(REGISTRY)/tvscreener-rs
APP_VERSION ?= $(shell awk '/^version = /{gsub(/"/, "", $$3); print $$3; exit}' Cargo.toml)
DOCKERFILE ?= docker/Dockerfile
DOCKER_BUILDKIT ?= 1

.DEFAULT_GOAL := help

.PHONY: help all verify \
	build build-release check \
	test test-lib test-offline test-live test-all \
	lint format format-check clippy \
	doc doc-open doc-clean \
	example-crypto example-manual \
	run run-mcp \
	audit deny regen-fields \
	docker-build docker-build-no-cache docker-push \
	docker-run docker-run-test docker-stop docker-inspect \
	clean

help:
	@echo "tvscreener-rs targets"
	@echo ""
	@echo "  make build           Debug build (lib + bins + examples)"
	@echo "  make build-release   Release build"
	@echo "  make check           cargo check --all-targets"
	@echo "  make test            Default test suite"
	@echo "  make test-live       Live e2e (TVSCREENER_LIVE=1)"
	@echo "  make test-all        Default suite + live"
	@echo "  make lint            fmt check + clippy"
	@echo "  make format          cargo fmt"
	@echo "  make verify          format-check + clippy + tests"
	@echo "  make doc             rustdoc → docs/api-rust/"
	@echo "  make doc-open        rustdoc + open in browser"
	@echo "  make doc-clean       remove docs/api-rust generated HTML"
	@echo "  make example-manual  Example: util / presets / display"
	@echo "  make example-crypto  Example: live crypto scan"
	@echo "  make run             CLI (bin tvscreener). ARGS='…' (default: --help)"
	@echo "                       e.g. make run ARGS='payload crypto --limit 2'"
	@echo "  make run-mcp         MCP server (bin tvscreener-mcp, --features mcp)"
	@echo "  make audit           cargo audit"
	@echo "  make deny            cargo deny check"
	@echo "  make regen-fields    Rebuild data/fields.json (needs PYTHON_ROOT=…)"
	@echo "                       e.g. make regen-fields PYTHON_ROOT=../tvscreener"
	@echo "  make docker-build    Build $(APP_IMAGE):$(TAG) (+ :$(APP_VERSION))"
	@echo "  make docker-push     Push image tags"
	@echo "  make docker-run      Compose prod up -d"
	@echo "  make docker-run-test Compose test up"
	@echo "  make docker-stop     Compose prod stop"
	@echo "  make docker-inspect  Local image tags / size"
	@echo "  make clean           cargo clean"
	@echo ""
	@echo "Overrides: CARGO_BIN=…  CARGO_FLAGS=…  TVSCREENER_LIVE=0|1  TVSCREENER_DEBUG=0|1"
	@echo "           REGISTRY=$(REGISTRY)  TAG=$(TAG)  APP_VERSION=$(APP_VERSION)  ARGS=…  PYTHON_ROOT=…"

all: verify

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

build:
	$(CARGO) build $(CARGO_FLAGS) --all-targets

build-release:
	$(CARGO) build $(CARGO_FLAGS) --release --all-targets

check:
	$(CARGO) check $(CARGO_FLAGS) --all-targets
	$(CARGO) check $(CARGO_FLAGS) --bins --examples

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

## Default suite (`cargo test`, then again with `--features mcp`).
test: test-offline

test-lib:
	$(CARGO) test $(CARGO_FLAGS) --lib -- --nocapture

test-offline:
	$(CARGO) test $(CARGO_FLAGS) -- --nocapture
	$(CARGO) test $(CARGO_FLAGS) --features mcp -- --nocapture

## Live HTTP e2e (`--features live`, serial).
test-live:
	TVSCREENER_LIVE=$(TVSCREENER_LIVE) $(CARGO) test $(CARGO_FLAGS) --features live --test e2e_live -- --test-threads=1 --nocapture

test-all: test-offline test-live

# ---------------------------------------------------------------------------
# Lint / format
# ---------------------------------------------------------------------------

CLIPPY_FLAGS := -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

format:
	$(CARGO) fmt $(NIGHTLY_FLAGS)

format-check:
	$(CARGO) fmt $(NIGHTLY_FLAGS) -- --check

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) --all-targets -- $(CLIPPY_FLAGS)
	$(CARGO) clippy $(CARGO_FLAGS) --all-targets --features live -- $(CLIPPY_FLAGS)
	$(CARGO) clippy $(CARGO_FLAGS) --all-targets --features mcp -- $(CLIPPY_FLAGS)

lint: format-check clippy

# ---------------------------------------------------------------------------
# Docs / examples / bins
# ---------------------------------------------------------------------------

## rustdoc → `docs/api-rust/`.
DOC_OUT ?= target/doc

doc:
	RUSTDOCFLAGS='-D warnings' $(CARGO) doc $(CARGO_FLAGS) --no-deps
	@test -d "$(DOC_OUT)" || (echo "missing $(DOC_OUT)"; exit 1)
	@rm -rf docs/api-rust
	@mkdir -p docs/api-rust
	@cp -a "$(DOC_OUT)/." docs/api-rust/
	@printf '%s\n' \
		'# Rust API documentation (rustdoc)' \
		'' \
		'Open [`tvscreener/index.html`](tvscreener/index.html).' \
		> docs/api-rust/README.md
	@echo "docs/api-rust/ updated - open docs/api-rust/tvscreener/index.html"

doc-open: doc
	@xdg-open docs/api-rust/tvscreener/index.html 2>/dev/null \
		|| open docs/api-rust/tvscreener/index.html 2>/dev/null \
		|| echo "Open docs/api-rust/tvscreener/index.html in a browser"

doc-clean:
	rm -rf docs/api-rust
	mkdir -p docs/api-rust
	@printf '%s\n' \
		'# Rust API documentation (rustdoc)' \
		'' \
		'Open [`tvscreener/index.html`](tvscreener/index.html) after `make doc`.' \
		> docs/api-rust/README.md

example-manual:
	$(CARGO) run $(CARGO_FLAGS) --example manual_test

example-crypto:
	$(CARGO) run $(CARGO_FLAGS) --example example_crypto

## Default CLI (`tvscreener`). Pass args with `ARGS=…`; empty ARGS → `--help`.
run:
	$(CARGO) run $(CARGO_FLAGS) --bin tvscreener -- $(if $(strip $(ARGS)),$(ARGS),--help)

## MCP stdio binary (`tvscreener-mcp`).
run-mcp:
	$(CARGO) run $(CARGO_FLAGS) --features mcp --bin tvscreener-mcp

# ---------------------------------------------------------------------------
# Supply chain / catalog
# ---------------------------------------------------------------------------

## Requires `cargo install cargo-audit`.
audit:
	$(CARGO) audit

## Requires `cargo install cargo-deny`.
deny:
	$(CARGO) deny check

## Rebuild `data/fields.json` + generated consts from a deepentropy/tvscreener clone.
## Example: `make regen-fields PYTHON_ROOT=../tvscreener`
regen-fields:
	@test -n "$(PYTHON_ROOT)" || (echo "set PYTHON_ROOT=/path/to/tvscreener"; exit 1)
	$(CARGO) run $(CARGO_FLAGS) --bin tvscreener -- regen-fields --python-root "$(PYTHON_ROOT)"

# ---------------------------------------------------------------------------
# Docker
# ---------------------------------------------------------------------------

COMPOSE_PROD ?= docker/docker-compose.prod.yml
COMPOSE_TEST ?= docker/docker-compose.test.yml

docker-build:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build \
		--network=host \
		--build-arg APP_VERSION=$(APP_VERSION) \
		-t $(APP_IMAGE):$(TAG) \
		-t $(APP_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE) \
		.

docker-build-no-cache:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build \
		--no-cache \
		--network=host \
		--build-arg APP_VERSION=$(APP_VERSION) \
		-t $(APP_IMAGE):$(TAG) \
		-t $(APP_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE) \
		.

docker-push:
	docker push $(APP_IMAGE):$(TAG)
	docker push $(APP_IMAGE):$(APP_VERSION)

docker-run:
	docker compose -f $(COMPOSE_PROD) up -d --force-recreate

docker-run-test:
	docker compose -f $(COMPOSE_TEST) up --no-build --force-recreate

docker-stop:
	docker compose -f $(COMPOSE_PROD) stop

docker-inspect:
	@docker image inspect $(APP_IMAGE):$(TAG) --format \
		'{{.RepoTags}} size={{.Size}} created={{.Created}}' 2>/dev/null \
		|| echo "Image $(APP_IMAGE):$(TAG) not found - run make docker-build"

# ---------------------------------------------------------------------------
# Verify / clean
# ---------------------------------------------------------------------------

## format-check + clippy + default test suite.
verify: format-check clippy test-offline
	@echo "verify OK"

clean:
	$(CARGO) clean
