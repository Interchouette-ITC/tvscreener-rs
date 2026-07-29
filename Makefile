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

HUB_IMAGE ?= gregoshop/tvscreener-rs
# personal GHCR mirror (GHCR_USERNAME / GHCR_PAT)
GHCR_PERSONAL_IMAGE ?= ghcr.io/groussac/tvscreener-rs
# Interchouette worker + org packages (GHCR_USERNAME_ITC / GHCR_PAT_ITC)
GHCR_WORKER_IMAGE ?= ghcr.io/interchouette/tvscreener-rs
GHCR_ORG_IMAGE ?= ghcr.io/interchouette-itc/tvscreener-rs
# Backward-compatible alias used by docker-build (Hub image).
REGISTRY ?= gregoshop
TAG ?= latest
APP_IMAGE = $(REGISTRY)/tvscreener-rs
APP_VERSION ?= $(shell awk '/^version = /{gsub(/"/, "", $$3); print $$3; exit}' Cargo.toml)
DOCKERFILE ?= docker/Dockerfile
DOCKER_BUILDKIT ?= 1
# Set CI=1 in GitHub Actions to skip interactive docker login.
CI ?= 0

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
	docker-build-dev docker-push-dev \
	docker-push-dev-hub docker-push-dev-ghcr-personal docker-push-dev-ghcr-itc \
	docker-push-release docker-push-release-hub \
	docker-push-release-ghcr-personal docker-push-release-ghcr-itc \
	docker-run docker-run-test docker-stop docker-inspect \
	version-show version-bump-patch version-bump-minor version-bump-major version-set \
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
	@echo "  make docker-build    Build $(HUB_IMAGE):$(TAG) (+ :$(APP_VERSION))"
	@echo "  make docker-build-dev  Build and tag :dev (Hub + 3 GHCR names)"
	@echo "  make docker-push-dev   Push :dev (local interactive logins)"
	@echo "  make docker-push-release  Tag release images (CI uses split push targets)"
	@echo "  make docker-push     Deprecated alias → prefer docker-push-dev / GitHub Release"
	@echo "  make docker-run      Compose prod up -d"
	@echo "  make docker-run-test Compose test up"
	@echo "  make docker-stop     Compose prod stop"
	@echo "  make docker-inspect  Local image tags / size"
	@echo "  make version-show    Print Cargo.toml version + suggested release tag"
	@echo "  make version-bump-patch|minor|major"
	@echo "  make version-set VERSION=x.y.z"
	@echo "  make clean           cargo clean"
	@echo ""
	@echo "Overrides: CARGO_BIN=…  CARGO_FLAGS=…  TVSCREENER_LIVE=0|1  TVSCREENER_DEBUG=0|1"
	@echo "           HUB_IMAGE=$(HUB_IMAGE)  APP_VERSION=$(APP_VERSION)  CI=0|1  ARGS=…  PYTHON_ROOT=…"

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
		-t $(HUB_IMAGE):$(TAG) \
		-t $(HUB_IMAGE):$(APP_VERSION) \
		-t $(APP_IMAGE):$(TAG) \
		-t $(APP_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE) \
		.

docker-build-no-cache:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build \
		--no-cache \
		--network=host \
		--build-arg APP_VERSION=$(APP_VERSION) \
		-t $(HUB_IMAGE):$(TAG) \
		-t $(HUB_IMAGE):$(APP_VERSION) \
		-t $(APP_IMAGE):$(TAG) \
		-t $(APP_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE) \
		.

## Build once and retag :dev on Hub + all GHCR names.
docker-build-dev:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build \
		--network=host \
		--build-arg APP_VERSION=$(APP_VERSION) \
		-t tvscreener-rs:dev \
		-t $(HUB_IMAGE):dev \
		-t $(GHCR_PERSONAL_IMAGE):dev \
		-t $(GHCR_WORKER_IMAGE):dev \
		-t $(GHCR_ORG_IMAGE):dev \
		-f $(DOCKERFILE) \
		.

docker-push-dev-hub:
	docker push $(HUB_IMAGE):dev

docker-push-dev-ghcr-personal:
	docker push $(GHCR_PERSONAL_IMAGE):dev

docker-push-dev-ghcr-itc:
	docker push $(GHCR_WORKER_IMAGE):dev
	docker push $(GHCR_ORG_IMAGE):dev

## Push :dev. Local: Hub then personal GHCR then ITC GHCR (re-login between). CI uses split targets.
docker-push-dev:
	@if [ "$(CI)" = "1" ]; then \
		echo "Use docker-push-dev-hub / docker-push-dev-ghcr-personal / docker-push-dev-ghcr-itc in CI"; \
		exit 1; \
	fi
	@echo "Logging in to Docker Hub..."; \
	docker login || { echo "Docker Hub login failed"; exit 1; }
	$(MAKE) docker-push-dev-hub
	@echo "Logging in to GHCR (personal)..."; \
	docker login ghcr.io || { echo "Skipping personal GHCR"; exit 0; }
	$(MAKE) docker-push-dev-ghcr-personal
	@echo "Logging in to GHCR (Interchouette / ITC)..."; \
	docker login ghcr.io || { echo "Skipping ITC GHCR"; exit 0; }
	$(MAKE) docker-push-dev-ghcr-itc

docker-push-release-hub:
	docker push $(HUB_IMAGE):$(APP_VERSION)
	docker push $(HUB_IMAGE):latest

docker-push-release-ghcr-personal:
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_PERSONAL_IMAGE):latest
	docker push $(GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker push $(GHCR_PERSONAL_IMAGE):latest

docker-push-release-ghcr-itc:
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_WORKER_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_WORKER_IMAGE):latest
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_ORG_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_ORG_IMAGE):latest
	docker push $(GHCR_WORKER_IMAGE):$(APP_VERSION)
	docker push $(GHCR_WORKER_IMAGE):latest
	docker push $(GHCR_ORG_IMAGE):$(APP_VERSION)
	docker push $(GHCR_ORG_IMAGE):latest

## Tag helpers for release workflow (push is split per registry login).
docker-push-release: docker-push-release-hub docker-push-release-ghcr-personal docker-push-release-ghcr-itc

## Deprecated for releases: Hub-only :$(TAG) + :$(APP_VERSION). Prefer docker-push-dev / GitHub Release.
docker-push:
	@echo "note: make docker-push is Hub-only; prefer make docker-push-dev or a GitHub Release"
	docker push $(HUB_IMAGE):$(TAG)
	docker push $(HUB_IMAGE):$(APP_VERSION)

docker-run:
	docker compose -f $(COMPOSE_PROD) up -d --force-recreate

docker-run-test:
	docker compose -f $(COMPOSE_TEST) up --no-build --force-recreate

docker-stop:
	docker compose -f $(COMPOSE_PROD) stop

docker-inspect:
	@docker image inspect $(HUB_IMAGE):$(TAG) --format \
		'{{.RepoTags}} size={{.Size}} created={{.Created}}' 2>/dev/null \
		|| docker image inspect $(HUB_IMAGE):dev --format \
			'{{.RepoTags}} size={{.Size}} created={{.Created}}' 2>/dev/null \
		|| echo "Image not found - run make docker-build or make docker-build-dev"

# ---------------------------------------------------------------------------
# Version (Cargo.toml) — verq-style helpers; release images via GitHub Release
# ---------------------------------------------------------------------------

version-show:
	@echo "Current version: $(APP_VERSION)"; \
	echo ""; \
	echo "Suggested GitHub Release tag:"; \
	echo "  v$(APP_VERSION)"; \
	echo ""; \
	echo "When creating a GitHub Release, use the Tag field (not only the title)."

version-bump-patch:
	@current="$(APP_VERSION)"; \
	new=$$(echo "$$current" | awk -F. '{print $$1"."$$2"."($$3+1)}'); \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	echo "Version bumped from $$current to $$new"

version-bump-minor:
	@current="$(APP_VERSION)"; \
	new=$$(echo "$$current" | awk -F. '{print $$1"."($$2+1)".0"}'); \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	echo "Version bumped from $$current to $$new"

version-bump-major:
	@current="$(APP_VERSION)"; \
	new=$$(echo "$$current" | awk -F. '{print ($$1+1)".0.0"}'); \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	echo "Version bumped from $$current to $$new"

version-set:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make version-set VERSION=x.y.z"; \
		exit 1; \
	fi; \
	current="$(APP_VERSION)"; \
	sed -i "s/^version = \"$$current\"/version = \"$(VERSION)\"/" Cargo.toml; \
	echo "Version set from $$current to $(VERSION)"

# ---------------------------------------------------------------------------
# Verify / clean
# ---------------------------------------------------------------------------

## format-check + clippy + default test suite.
verify: format-check clippy test-offline
	@echo "verify OK"

clean:
	$(CARGO) clean
