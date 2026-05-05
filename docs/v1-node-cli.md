# agent-wire-substrate-node V1 CLI

`agent-wire-substrate-node` is the pure-CLI reference participant for the
Agent Wire substrate. It is intentionally small: protocol strings enter at the
CLI, HTTP, or MCP edge, then immediately convert into foundation-owned typed
surfaces before the rest of the node sees them.

## Install Or Build

From the workspace root:

```sh
cargo build --release
target/release/agent-wire-substrate-node --help
```

During development, run through Cargo:

```sh
cargo run -p agent-wire-substrate-node -- surface
```

## Surface Discovery

```sh
agent-wire-substrate-node surface
agent-wire-substrate-node mcp manifest
agent-wire-substrate-node http manifest
agent-wire-substrate-node maintenance run-once
```

These commands print the V1 CLI, MCP, HTTP, and maintenance surfaces. The MCP
and HTTP manifests are built from `McpTool::ALL` and `HttpRoute::ALL` in
`agent-wire-foundation::vocabulary::canonical_ops`.

## Identity

```sh
agent-wire-substrate-node identity signup
agent-wire-substrate-node identity login
agent-wire-substrate-node identity status
agent-wire-substrate-node identity persist ~/.wire-node/state
agent-wire-substrate-node identity load ~/.wire-node/state
```

`signup`, `login`, and `status` expose the typed protocol bindings. `persist`
and `load` exercise the local V1 node identity store. The persisted document is
`identity_snapshot/v1-identity.md` under the selected state directory and
contains only compact node identity state: node id, operator handle, namespace,
master key id, and endpoints.

Live mainnet authentication remains the separate command:

```sh
agent-wire-substrate-node auth
```

It uses `WIRE_API_TOKEN`, `WIRE_API_TOKEN_FILE`, `WIRE_DEVICE_SECRET`, or
`WIRE_OPERATOR_EMAIL` as bootstrap input and persists validated auth state in
`~/.wire-node/state/mainnet_auth_credential/agent-wire-substrate-node.md`.
The auth loader can migrate the legacy JSON auth file after validating it.

## Chain Compile And Execute

```sh
agent-wire-substrate-node chain compile chain.yaml
agent-wire-substrate-node chain quote chain.yaml
agent-wire-substrate-node chain execute chain.yaml trusted
```

The loader accepts canonical Wire action JSON/YAML first, then falls back to the
Rust-native internal test shape. Canonical action chains should use
GoodNewsEveryone-style fields such as `schemaVersion`, `actionType`, `steps`,
`tool`, `outputSchema`, `modelTier`, `forEach`, `actionId`, and `waitFor`.

Minimal canonical chain:

```yaml
schemaVersion: 1
actionType: chain
permissions:
  contribute: true
  maxCost: 1000
steps:
  - name: publish
    operation: wire
    tool: wire.contribute
    outputSchema:
      type: object
```

Execution is local routing in V1: LLM steps route to compute-market execution
adapters, Wire steps route to typed HTTP/MCP protocol bindings, Task steps
route to task-board bindings, and Game returns the explicit out-of-V1 stub.

## Compute Market

```sh
agent-wire-substrate-node compute offer
agent-wire-substrate-node compute quote
agent-wire-substrate-node compute purchase
agent-wire-substrate-node compute fill
agent-wire-substrate-node compute jobs
agent-wire-substrate-node compute market-surface
```

These commands print typed protocol bindings for compute-market participation.
The deterministic market state machine lives in `agent-wire-compute-market`.
Live provider/requester settlement evidence is covered by D3/L6 validation
commands and remains environment-gated.

## MCP And HTTP Runtime Dispatch

```sh
agent-wire-substrate-node mcp dispatch wire_pulse
agent-wire-substrate-node http dispatch GET /wire/pulse
agent-wire-substrate-node http smoke
```

`mcp dispatch` maps a tool name once into `McpTool`. `http dispatch` maps a
method/path once into `HttpRoute`, including templated routes such as
`/wire/messages/{messageIdentifier}`. `http smoke` runs a real local loopback
single-request listener smoke for `GET /wire/pulse`.

These are listener skeletons for V1 validation, not a production long-running
server.

## Maintenance Scheduler

```sh
agent-wire-substrate-node maintenance run-once
agent-wire-substrate-node maintenance schedule-tick
```

`run-once` dispatches the 12 maintenance surfaces. `schedule-tick` runs the
typed scheduler: the 8 local V1 tasks fire when due, and the 4 future/deferred
tasks are logged and skipped.

## Combined Runtime Smoke

```sh
agent-wire-substrate-node runtime smoke ~/.wire-node/state
scripts/v1-node-surface.sh
```

The combined smoke exercises:

- surface manifests
- MCP dispatch
- HTTP dispatch
- loopback HTTP listener smoke
- identity persist/load
- maintenance run-once
- maintenance scheduler tick

It does not deploy, mutate a live DB, publish packages, or start the L6 calendar
window.

## Live Validation Commands

Environment-gated commands:

```sh
agent-wire-substrate-node contribution-sync
agent-wire-substrate-node layer5-live-llm
agent-wire-substrate-node d3-live-compute-settlement
agent-wire-substrate-node l6-stability-driver
agent-wire-substrate-node l6-failure-injection
```

Use these only when the required credentials and live-service fences are
explicitly in scope. Keep real-money amounts small, and do not infer deploy or
publish authority from a green local smoke.
