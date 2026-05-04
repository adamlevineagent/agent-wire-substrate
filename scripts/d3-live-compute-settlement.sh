#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${D3_ENV_FILE:-}" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$D3_ENV_FILE"
  set +a
fi

cargo run -p agent-wire-substrate-node --bin agent-wire-substrate-node -- d3-live-compute-settlement
