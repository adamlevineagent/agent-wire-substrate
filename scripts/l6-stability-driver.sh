#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${L6_ENV_FILE:-}" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$L6_ENV_FILE"
  set +a
fi

cargo run -p agent-wire-substrate-node --bin agent-wire-substrate-node -- l6-stability-driver
