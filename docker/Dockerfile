#
# Multi-binary image: tvscreener, tvscreener-mcp, tvscreener-tui.
# Default: TUI + MCP HTTP on :8787 (see tvscreener-entrypoint).
# Build context: repository root.
#
#   docker build -f docker/Dockerfile -t interchouette/tvscreener-rs:latest .
#   make docker-build && make docker-push
#
# Builder: Debian 13 (trixie). Runtime: distroless cc-debian13 (no apt/perl/shell).
# Binary is stripped; rustls needs no OpenSSL packages.

ARG RUST_IMAGE=rust:slim-trixie
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian13:nonroot

# ---------------------------------------------------------------------------
# Builder - release binaries (deps cached separately from app sources)
# ---------------------------------------------------------------------------
FROM ${RUST_IMAGE} AS builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    binutils \
    pkg-config \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV CARGO_TERM_COLOR=never \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    CARGO_INCREMENTAL=0

COPY Cargo.toml Cargo.lock ./

# Warm dependency layer (dummy crate; real sources overwrite next).
RUN mkdir -p src/bin/tvscreener data \
  && printf 'fn main() {}\n' > src/bin/tvscreener/main.rs \
  && printf 'fn main() {}\n' > src/bin/tvscreener_mcp.rs \
  && printf 'fn main() {}\n' > src/bin/tvscreener_tui.rs \
  && printf 'fn main() {}\n' > src/bin/tvscreener_entrypoint.rs \
  && printf '// docker dep cache\n' > src/lib.rs \
  && printf '{}\n' > data/fields.json \
  && cargo build --release --locked \
       --bin tvscreener --bin tvscreener-mcp --bin tvscreener-tui \
       --bin tvscreener-entrypoint \
  && rm -rf src data target/release/deps/tvscreener* \
       target/release/.fingerprint/tvscreener* \
       target/release/tvscreener \
       target/release/tvscreener.d \
       target/release/tvscreener-mcp \
       target/release/tvscreener-mcp.d \
       target/release/tvscreener-tui \
       target/release/tvscreener-tui.d \
       target/release/tvscreener-entrypoint \
       target/release/tvscreener-entrypoint.d

COPY data ./data
COPY src ./src

RUN touch src/lib.rs \
      src/bin/tvscreener/main.rs \
      src/bin/tvscreener_mcp.rs \
      src/bin/tvscreener_tui.rs \
      src/bin/tvscreener_entrypoint.rs \
  && cargo build --release --locked \
       --bin tvscreener --bin tvscreener-mcp --bin tvscreener-tui \
       --bin tvscreener-entrypoint \
  && strip -s target/release/tvscreener \
       target/release/tvscreener-mcp \
       target/release/tvscreener-tui \
       target/release/tvscreener-entrypoint \
  && cp target/release/tvscreener /tvscreener \
  && cp target/release/tvscreener-mcp /tvscreener-mcp \
  && cp target/release/tvscreener-tui /tvscreener-tui \
  && cp target/release/tvscreener-entrypoint /tvscreener-entrypoint

# ---------------------------------------------------------------------------
# Runtime - distroless Debian 13, non-root, CA certs for HTTPS
# ---------------------------------------------------------------------------
FROM ${RUNTIME_IMAGE} AS runtime

ARG APP_VERSION=1.0.0

WORKDIR /app

COPY --from=builder /tvscreener /usr/local/bin/tvscreener
COPY --from=builder /tvscreener-mcp /usr/local/bin/tvscreener-mcp
COPY --from=builder /tvscreener-tui /usr/local/bin/tvscreener-tui
COPY --from=builder /tvscreener-entrypoint /usr/local/bin/tvscreener-entrypoint
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

USER nonroot:nonroot

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt \
    SSL_CERT_DIR=/etc/ssl/certs \
    ENABLE_MCP=1 \
    TVSCREENER_MCP_ADDR=0.0.0.0:8787

EXPOSE 8787

LABEL org.opencontainers.image.title="tvscreener-rs" \
      org.opencontainers.image.description="Unofficial TradingView screener API client (Rust) - CLI, TUI, and MCP" \
      org.opencontainers.image.vendor="Interchouette" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${APP_VERSION}" \
      org.opencontainers.image.source="https://github.com/Interchouette-ITC/tvscreener-rs"

ENTRYPOINT ["/usr/local/bin/tvscreener-entrypoint"]
