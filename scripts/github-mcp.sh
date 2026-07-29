#!/usr/bin/env bash
# Launch official GitHub MCP (Docker). Token from a local KEY=value creds file.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CREDS="${GITHUB_CREDS_FILE:-${ROOT}/.github_credentials}"

if [[ ! -f "${CREDS}" ]]; then
  echo "missing creds file: ${CREDS}" >&2
  echo "set GITHUB_CREDS_FILE or create ${ROOT}/.github_credentials with GITHUB_TOKEN=…" >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
# shellcheck source=/dev/null
source <(
  awk '
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^GITHUB_TOKEN=/ {
      sub(/^GITHUB_TOKEN=/, "GITHUB_PERSONAL_ACCESS_TOKEN=")
      print
      next
    }
    /^GITHUB_PERSONAL_ACCESS_TOKEN=/ { print; next }
  ' "${CREDS}"
)
set +a

if [[ -z "${GITHUB_PERSONAL_ACCESS_TOKEN:-}" ]]; then
  echo "GITHUB_TOKEN missing in ${CREDS} (KEY=value)" >&2
  exit 1
fi

export GITHUB_PERSONAL_ACCESS_TOKEN

if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
  echo "docker required for ghcr.io/github/github-mcp-server" >&2
  exit 1
fi

exec docker run -i --rm \
  -e GITHUB_PERSONAL_ACCESS_TOKEN \
  ghcr.io/github/github-mcp-server
