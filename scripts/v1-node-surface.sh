#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

: "${CARGO_HOME:=/private/tmp/codex-kramer-cargo-home}"
export CARGO_HOME

cargo run -p agent-wire-substrate-node -- surface >/dev/null
cargo run -p agent-wire-substrate-node -- mcp manifest >/dev/null
cargo run -p agent-wire-substrate-node -- mcp dispatch wire_pulse >/dev/null
cargo run -p agent-wire-substrate-node -- http manifest >/dev/null
cargo run -p agent-wire-substrate-node -- http dispatch GET /wire/pulse >/dev/null
cargo run -p agent-wire-substrate-node -- http smoke >/dev/null
cargo run -p agent-wire-substrate-node -- maintenance run-once >/dev/null
cargo run -p agent-wire-substrate-node -- maintenance schedule-tick >/dev/null
RUNTIME_STATE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/agent-wire-v1-runtime.XXXXXX")"
trap 'rm -rf "$RUNTIME_STATE_DIR"' EXIT
cargo run -p agent-wire-substrate-node -- identity persist "$RUNTIME_STATE_DIR" >/dev/null
cargo run -p agent-wire-substrate-node -- identity load "$RUNTIME_STATE_DIR" >/dev/null
cargo run -p agent-wire-substrate-node -- runtime smoke "$RUNTIME_STATE_DIR" >/dev/null

echo "V1 node surface and runtime smoke passed."
