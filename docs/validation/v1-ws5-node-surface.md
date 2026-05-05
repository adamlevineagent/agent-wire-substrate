# V1 WS5 Node Surface Slice

This slice starts the WS5 hot zone by moving the node crate from a validation
binary into a typed node-surface dispatcher.

## References

- Mission: `playful/124/15`
- Op manifest: `playful/123/106`
- Gap analysis: `playful/123/93`
- WS3 compiler slice: `docs/validation/v1-ws3-compiler.md`
- WS-Compute state-machine slice: `docs/validation/v1-ws-compute-state-machine.md`

## Shipped Surface

- `agent-wire-foundation::vocabulary::canonical_ops` now includes sealed
  registry entries for the canonical MCP tool surface and the V1 HTTP route
  surface.
- `agent-wire-substrate-node::v1_surface` exposes typed CLI, MCP, HTTP, and
  maintenance manifests.
- CLI additions:
  - `surface`
  - `identity signup|login|status`
  - `chain compile <chain.yaml|json>`
  - `chain execute <chain.yaml|json>`
  - `chain quote <chain.yaml|json>`
  - `compute offer|quote|purchase|fill|jobs|market-surface`
  - `mcp manifest`
  - `http manifest`
  - `maintenance run-once`
- The maintenance runner fires the 8 local V1 tasks and records the 4
  future/deferred tasks as typed stubs.
- Chain execution is still local routing, not live side effects: LLM steps route
  to compute-market, Wire steps route to typed HTTP/MCP bindings, Task steps
  route to task-board bindings, and Game remains the explicit out-of-V1 stub.

## Fences

- Protocol strings live at the protocol edge only, through foundation-owned
  typed registry values.
- Node business logic consumes `McpTool`, `HttpRoute`, `MaintenanceTask`, and
  compiled-operation enums.
- No runtime HTTP server, MCP network listener, live Wire calls, deploy, npm
  publish, crates.io publish, or destructive DB operation is included in this
  slice.

## Validation

- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-foundation vocabulary`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate-node v1`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo run -p agent-wire-substrate-node -- surface`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home scripts/v1-node-surface.sh`

Full workspace validation is required before the slice is reported complete.
