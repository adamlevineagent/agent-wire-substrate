# V1 WS5 Runtime Listener Slice

This slice extends the WS5 protocol surface into a local runtime shell for the
reference substrate node.

## Scope

- HTTP listener skeleton:
  - accepts an HTTP method/path at the protocol edge
  - maps it exactly once into foundation-owned `HttpRoute`
  - routes through the existing typed `dispatch_http_route` path
  - includes a loopback single-request smoke for `/wire/pulse`
- MCP listener skeleton:
  - accepts a tool name at the protocol edge
  - maps it exactly once into foundation-owned `McpTool`
  - routes through the existing typed `dispatch_mcp_tool` path
- Identity persistence:
  - atomically writes a compact V1 node identity state wire-native document
  - reads the same document back through the typed foundation codec
  - keeps the persisted state to node/operator/namespace/key/endpoints only
- Maintenance scheduler runtime:
  - schedules every foundation-owned `MaintenanceTask`
  - fires local V1 tasks when due
  - records deferred future tasks without executing them

## Fences

- No deploy, npm publish, crates.io publish, live DB mutation, or live Wire
  writes are part of this validation slice.
- HTTP/MCP protocol-edge strings are translated once into sealed foundation
  enums. Business logic continues to receive typed routes/tools.
- The scheduler is local deterministic runtime plumbing; deferred tasks remain
  logged and skipped in V1.

## Validation

Before report closeout, run:

- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo fmt --all -- --check`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate-node v1_runtime`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate-node v1`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home scripts/v1-node-surface.sh`
- `test -x target/release/agent-wire-substrate-node`
- `git diff --check`
- `git diff --cached --check`
- sealed-registry string-dispatch probe: no hits
