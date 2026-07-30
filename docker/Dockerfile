#
# Size-optimized multi-stage image for tvscreener-mcp (stdio MCP entrypoint).
# Build context: repository root.
#
#   docker build -f docker/Dockerfile -t gregoshop/tvscreener-rs:latest .
#   make docker-build && make docker-push
#
# Runtime is debian:bookworm-slim (ca-certificates only) for Hub availability.
# Binary is stripped; rustls needs no OpenSSL packages.

ARG RUST_IMAGE=rust:bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

# ---------------------------------------------------------------------------
# Builder - release binary only (deps cached separately from app sources)
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
RUN mkdir -p src/bin data \
  && printf 'fn main() {}\n' > src/bin/tvscreener_mcp.rs \
  && printf '// docker dep cache\n' > src/lib.rs \
  && printf '{}\n' > data/fields.json \
  && cargo build --release --locked --bin tvscreener-mcp \
  && rm -rf src data target/release/deps/tvscreener* \
       target/release/.fingerprint/tvscreener* \
       target/release/tvscreener-mcp \
       target/release/tvscreener-mcp.d

COPY data ./data
COPY src ./src

RUN touch src/lib.rs src/bin/tvscreener_mcp.rs \
  && cargo build --release --locked --bin tvscreener-mcp \
  && strip -s target/release/tvscreener-mcp \
  && cp target/release/tvscreener-mcp /tvscreener-mcp

# ---------------------------------------------------------------------------
# Runtime - minimal Debian, non-root, CA certs for HTTPS
# ---------------------------------------------------------------------------
FROM ${RUNTIME_IMAGE} AS runtime

ARG APP_VERSION=1.0.0

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && groupadd --system --gid 65532 nonroot \
  && useradd --system --uid 65532 --gid nonroot --home /app --shell /usr/sbin/nologin nonroot \
  && mkdir -p /app \
  && chown nonroot:nonroot /app

WORKDIR /app

COPY --from=builder --chown=nonroot:nonroot /tvscreener-mcp /usr/local/bin/tvscreener-mcp

USER nonroot:nonroot

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt \
    SSL_CERT_DIR=/etc/ssl/certs

LABEL org.opencontainers.image.title="tvscreener-rs" \
      org.opencontainers.image.description="Unofficial TradingView screener API client (Rust port) - MCP stdio binary" \
      org.opencontainers.image.vendor="gregoshop" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${APP_VERSION}" \
      org.opencontainers.image.source="https://github.com/Interchouette-ITC/tvscreener-rs"

ENTRYPOINT ["/usr/local/bin/tvscreener-mcp"]
