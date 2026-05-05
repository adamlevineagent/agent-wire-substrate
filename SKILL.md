---
name: agent-wire-substrate
description: Use when consuming or modifying the agent-wire-substrate Rust workspace: foundation/contracts/transport/market crates, substrate node composition, downstream app integration, and the locked architecture boundaries.
type: docs-skill
---

# agent-wire-substrate Skill

Use this skill when a fresh agent needs to understand or consume the
`agent-wire-substrate` repo without first reading the canonical project-docs
stack. This repo is the substrate-tier library workspace for Wire v2 verticals.
It is not itself a deployment.

## What This Is

`agent-wire-substrate` is a Rust Cargo workspace of libraries plus a reference
node binary. It packages the foundation layer, bilateral contract boundary,
canonical Wire action compiler, Cloudflare transport driver, and three neutral
market surfaces: compute, storage, and relay.

Downstream applications such as pyramids v2, kitty-wire, future Sovereign
Graphs, and reference clients should consume the substrate through these crates
instead of copying Wire deployment code. Before publication, use workspace path
dependencies. After publication, the intended consumer shape is:

```sh
cargo add agent-wire-substrate
```

Treat the substrate as reusable chassis code. Deployment-specific databases,
JWT issuers, market provider registration, fleet wiring, UI, and vertical
business rules stay outside this repo.

## Crate Layout

The workspace has nine crates:

| Crate | Responsibility |
| --- | --- |
| `agent-wire-contracts` | Bilateral DTO crate and `ContractWrap<T>` boundary shapes. |
| `agent-wire-foundation` | Identity, refs, namespace, transport traits, sandbox, vocabulary mechanism, economics, events, contracts boundary, and dependency guard. |
| `agent-wire-compiler` | Canonical Wire action compiler IR, sealed op manifests, canonical GoodNewsEveryone action DTO adapters, and quote/review/trusted compile modes. |
| `agent-wire-transport-cloudflare` | Cloudflare tunnel driver implementing foundation transport traits. |
| `agent-wire-compute-market` | Neutral compute-market contracts and provider/requester scaffolds. |
| `agent-wire-storage-market` | Greenfield storage-market trait scaffolds. |
| `agent-wire-relay-market` | Greenfield relay-market trait scaffolds. |
| `agent-wire-substrate` | Publishable umbrella composition library over the lower substrate crates. |
| `agent-wire-substrate-node` | Reference CLI binary: V1 identity, chain, compute, MCP, HTTP, runtime listener, scheduler, auth/sync/D3/L6 validation surfaces. |

Dependency direction matters:

- `contracts` is the DTO base layer.
- `foundation` may wrap DTOs from `contracts`.
- Compiler, transport, and market crates may depend on `foundation` and
  `contracts` as needed.
- `substrate` and `node` may compose all substrate crates.
- `foundation` must not import market crates, node, Cloudflare, Tauri, pyramid,
  or deployment code.

The guard for this is `crates/foundation/src/dependency_guard.rs`, plus
workspace tests in `crates/node/tests/`. If a change weakens the dependency
direction, fix the architecture instead of bypassing the guard.

## Locked Architectural Decisions

These decisions are stable substrate commitments. Do not reopen them in local
consumer work unless Partner/@playful explicitly routes a design revision.

1. Option 3 WRAP: the contracts boundary uses wrapped DTOs and explicit
   conversions. Runtime values cross the boundary through `ContractWrap<T>` and
   `From`/boundary conversion code, not inherent-method drift.
2. Sandbox-foundation-module: sandbox capability and policy primitives live in
   `foundation`, not in pyramid or vertical application crates.
3. Cross-graph identity: master public key, signed handle claim, private alias,
   mid-slug ref parsing, reputation snapshot, and firewall primitives are
   foundation concepts.
4. Vocabulary mechanism only: foundation provides bounded vocabulary terms and
   resolver traits. Static vocabulary maps belong to deployments.
5. `TunnelUrl` pre-driver: endpoint types live in foundation before any
   concrete transport driver.
6. Neutral compute contracts: compute-market imports foundation/contracts only;
   it must not import pyramid, app, or GoodNewsEveryone runtime code.
7. Storage and relay are greenfield substrate scaffolds. Do not backfill legacy
   pyramid behavior into them.
8. Boot/server split: node composition owns startup and operator API surfaces;
   foundation remains primitive-only.
9. Dependency-direction tests are part of the architecture. Keep no-pyramid,
   no-Tauri, no-Cloudflare-in-foundation, and no-deployment imports green.
10. Track B reframe: this repo is the v2 substrate chassis and prerequisite
    stack for downstream verticals, not the pyramid v2 app itself.

## How To Consume

Before crates are published, consume from a local workspace or path dependency:

```toml
[dependencies]
agent-wire-foundation = { path = "../agent-wire-substrate/crates/foundation" }
agent-wire-contracts = { path = "../agent-wire-substrate/crates/contracts" }
agent-wire-compiler = { path = "../agent-wire-substrate/crates/compiler" }
agent-wire-compute-market = { path = "../agent-wire-substrate/crates/compute-market" }
agent-wire-storage-market = { path = "../agent-wire-substrate/crates/storage-market" }
agent-wire-relay-market = { path = "../agent-wire-substrate/crates/relay-market" }
agent-wire-transport-cloudflare = { path = "../agent-wire-substrate/crates/transport-cloudflare" }
agent-wire-substrate = { path = "../agent-wire-substrate/crates/substrate" }
agent-wire-substrate-node = { path = "../agent-wire-substrate/crates/node" }
```

Post-publication, prefer the umbrella package once it exists:

```sh
cargo add agent-wire-substrate
```

Use the smallest crate surface that fits the consumer:

- Application logic needing refs, identity, sandbox, vocabulary, economics, or
  transport traits should depend on `agent-wire-foundation`.
- DTO interop should depend on `agent-wire-contracts` and keep the WRAP
  boundary explicit.
- Canonical action chain compilation should depend on `agent-wire-compiler`.
  External GoodNewsEveryone-shaped action JSON/YAML crosses the boundary
  through canonical DTO adapters; internal execution remains typed.
- Market integrations should consume `agent-wire-compute-market`,
  `agent-wire-storage-market`, or `agent-wire-relay-market` without importing
  vertical deployment code.
- Full reference-node composition should consume `agent-wire-substrate-node`.

## Composition Entrypoint

The primary composition surface is:

```rust
use agent_wire_substrate_node::{compose_substrate_node, NodeConfig, NodeRuntime};

let runtime: NodeRuntime = compose_substrate_node(NodeConfig::demo()?)?;
```

`compose_substrate_node(NodeConfig) -> NodeRuntime` returns the configured
transport session, lifecycle worker set, operator API surface, vocabulary entry,
and all three market bundles wired together.

This is the substrate composition root. It is useful for reference clients,
synthetic validation, and downstream app bootstrapping. It is not a production
Wire deployment by itself.

## V1 Reference CLI

`agent-wire-substrate-node` is a pure CLI reference participant. The operator
surface is documented in `docs/v1-node-cli.md`; deployment and release gates are
documented in `docs/v1-node-deployment.md`.

Important commands:

```sh
agent-wire-substrate-node surface
agent-wire-substrate-node identity signup
agent-wire-substrate-node identity persist ~/.wire-node/state
agent-wire-substrate-node chain compile chain.yaml
agent-wire-substrate-node chain execute chain.yaml trusted
agent-wire-substrate-node compute offer
agent-wire-substrate-node mcp manifest
agent-wire-substrate-node mcp dispatch wire_pulse
agent-wire-substrate-node http manifest
agent-wire-substrate-node http dispatch GET /wire/pulse
agent-wire-substrate-node maintenance schedule-tick
agent-wire-substrate-node runtime smoke
```

The HTTP/MCP runtime surfaces translate protocol-edge strings once into
foundation-owned sealed enums (`HttpRoute` and `McpTool`). Do not add downstream
business logic that switches on raw op names.

## Standing Principles

1. Ship the fence with the primitive. If a primitive requires a boundary,
   idempotency rule, recovery policy, or namespace guard, include the guard in
   the same substrate layer instead of leaving consumers to remember it.
2. Keep sealed-registry packaging honest. Reserved primitive names and
   registry-like surfaces should be encoded as bounded constructors or explicit
   types, not stringly deployment convention.
3. State both property-tested and property-not-tested coverage in validation
   docs. Synthetic green is not live green; every harness should say what it
   proves and what remains outside scope.
4. Prefer deterministic harnesses before live smoke. L3/L4 are in-memory
   substrate checks; L5/L6 advance into live-provider and repeated D3-style
   settlement validation only when required env and services are present.
5. Fail closed around settlement, claim, and recovery semantics. Duplicate
   claims, duplicate completions, duplicate settlements, malformed refs, and
   invariant leaks are substrate bugs, not consumer taste.

## What This Is Not

This repo is not:

- a Wire production deployment;
- the GoodNewsEveryone app;
- pyramids v2;
- kitty-wire;
- a UI;
- a Supabase schema authority;
- a place for vertical-specific contribution types or app policy;
- a replacement for live deployment validation.

Foundation and market crates should remain reusable substrate code. If a change
needs deployment secrets, live database assumptions, route handlers, or app
policy, it probably belongs in a downstream app or deployment repo.

## Locked vs Open

Stable for consumers:

- the nine-crate workspace shape: eight library crates plus the reference-node
  binary crate;
- contracts/foundation/transport/market/node dependency direction;
- Option 3 WRAP boundary;
- foundation ownership of identity, refs, sandbox, vocabulary mechanism,
  economics, event envelope, and transport trait primitives;
- compute/storage/relay as neutral market surfaces;
- `compose_substrate_node(NodeConfig) -> NodeRuntime` as the reference
  composition entrypoint;
- V1 CLI/runtime surfaces as typed protocol-edge adapters over sealed
  registries, not deployment policy;
- validation language that separates property tested from property not tested.

Still in flux:

- crate publication and final umbrella-package layout;
- production deployment wiring;
- bundled-cloudflared release proof and live integration settlement evidence;
- live bridge/runtime behavior for Sovereign Graph peering;
- long-running L6 evidence windows and failure-injection coverage;
- downstream vertical policies for pyramids v2, kitty-wire, and future apps.

When in doubt, preserve the substrate boundary and make downstream consumers
adapt to it. Do not make the substrate import its consumers.
