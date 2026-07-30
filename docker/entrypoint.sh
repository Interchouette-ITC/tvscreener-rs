#!/bin/sh
# Copyright 2026 tvscreener-rs contributors
# SPDX-License-Identifier: Apache-2.0
#
# Docker entrypoint: MCP HTTP alone, TUI±MCP, interactive CLI±MCP, or one-shot CLI.
set -eu

MCP_PID=""
MCP_ADDR="${TVSCREENER_MCP_ADDR:-0.0.0.0:8787}"

cleanup() {
  if [ -n "${MCP_PID}" ]; then
    kill "${MCP_PID}" 2>/dev/null || true
    wait "${MCP_PID}" 2>/dev/null || true
  fi
}

trap cleanup EXIT INT TERM

mcp_enabled() {
  case "${ENABLE_MCP:-1}" in
    0|false|FALSE|no|NO|off|OFF) return 1 ;;
    *) return 0 ;;
  esac
}

start_mcp_http() {
  if ! mcp_enabled; then
    return 0
  fi
  tvscreener-mcp --http --listen "${MCP_ADDR}" &
  MCP_PID=$!
}

# True when the foreground should keep the container alive (TUI / interactive CLI).
is_long_lived() {
  if [ "$#" -eq 0 ]; then
    return 0
  fi

  cmd="$1"
  shift
  case "${cmd}" in
    tvscreener-tui | /usr/local/bin/tvscreener-tui)
      return 0
      ;;
    tvscreener | /usr/local/bin/tvscreener)
      while [ "$#" -gt 0 ]; do
        case "$1" in
          --debug)
            shift
            ;;
          --help | -h)
            return 1
            ;;
          -*)
            shift
            ;;
          *)
            break
            ;;
        esac
      done
      # No subcommand → interactive CLI.
      [ "$#" -eq 0 ]
      return
      ;;
    *)
      return 1
      ;;
  esac
}

has_tty() {
  [ -t 0 ] || [ -t 1 ]
}

# Explicit command override (CLI / TUI / MCP / other).
if [ "$#" -gt 0 ]; then
  if is_long_lived "$@"; then
    start_mcp_http
  fi
  exec "$@"
fi

# No args: detached / no TTY → MCP HTTP only.
if ! has_tty; then
  exec tvscreener-mcp --http --listen "${MCP_ADDR}"
fi

# Interactive TTY → TUI (+ MCP unless disabled).
start_mcp_http
exec tvscreener-tui crypto --limit 25
