#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

: "${CARGO_HOME:=/private/tmp/codex-kramer-cargo-home}"
export CARGO_HOME

cargo run -p agent-wire-substrate-node -- surface >/dev/null
cargo run -p agent-wire-substrate-node -- mcp manifest >/dev/null
cargo run -p agent-wire-substrate-node -- http manifest >/dev/null
cargo run -p agent-wire-substrate-node -- maintenance run-once >/dev/null

echo "V1 node surface manifest and maintenance smoke passed."
