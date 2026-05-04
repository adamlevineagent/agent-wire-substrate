#!/usr/bin/env bash
set -euo pipefail

cargo run -p agent-wire-substrate-node --bin agent-wire-substrate-node -- l6-failure-injection
