#!/usr/bin/env bash
set -euo pipefail

cargo run -p agent-wire-node --bin agent-wire-node -- layer4-synthetic
