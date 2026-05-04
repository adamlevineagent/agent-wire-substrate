#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${LAYER5_ENV_FILE:-}" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$LAYER5_ENV_FILE"
  set +a
fi

cargo run -p agent-wire-substrate-node --bin agent-wire-substrate-node -- layer5-live-llm
