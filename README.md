# agent-wire-substrate

The substrate-tier of the Wire: foundation crate, transport drivers, and the
three markets (compute, storage, relay), packaged as a Cargo workspace and
composed by the `agent-wire-substrate-node` V1 reference binary.

This is the chassis. Verticals such as mainnet Wire, kitty-wire, and future
Sovereign Graphs deploy on top of it. Pyramid-app-v2 (Track B) eventually
migrates onto it.

## What This Repo Is

`agent-wire-substrate` is a Rust Cargo workspace with nine crates: eight
publishable library crates plus the reference-node binary crate.

| Crate | Role |
| --- | --- |
| `foundation` | Identity and cross-graph types, refs, namespace, transport trait, sandbox, vocabulary mechanism, economics primitives, event envelope, contracts boundary, dependency guard |
| `contracts` | Bilateral DTO crate; `From<ContractDto>` conversions only, with no inherent methods (Option 3 WRAP) |
| `compiler` | Canonical Wire action compiler IR, sealed op manifests, canonical GoodNewsEveryone action DTO adapters, and quote/review/trusted compile modes |
| `transport-cloudflare` | Cloudflare driver implementing the foundation transport trait (P2P tunnel, default 12h rotation) |
| `compute-market` | Neutral compute-market contracts (`ComputeJobEnvelope`, `ModelInvocation`, `ExecutionAdapter`, `DeliveryPolicy`, `EventSink`, `ChronicleSink`, `QueueAdmission`, `DispatchPolicy`, `ComputeJobContract`) plus provider/requester scaffolds |
| `storage-market` | Storage-market trait scaffold (greenfield) |
| `relay-market` | Relay-market trait scaffold (greenfield) |
| `substrate` | Publishable umbrella composition library over the lower substrate crates |
| `node` | CLI validation and reference-node binary shell; exposes V1 identity, chain, compute, MCP, HTTP, runtime, and maintenance surfaces |

## Architectural Commitments

The substrate codifies ten decisions made during the substrate-tier design
session on 2026-05-03 and 2026-05-04:

1. **Option 3 WRAP** for contracts boundary: foundation wraps the contracts
   crate via explicit `From<ContractDto>` conversions; no inherent-method drift
   across the bilateral boundary.
2. **Sandbox-foundation-module**: the sandbox primitive lives in foundation, not
   pyramid-side.
3. **Cross-graph identity primitives** in foundation: master ed25519 pubkey,
   email recovery, handle canonicalization, mid-slug, and reputation-firewall
   types.
4. **Vocabulary mechanism only**: no pyramid static maps in foundation;
   vocabulary resolves per deployment.
5. **TunnelUrl pre-driver**: the transport endpoint type lives in foundation;
   drivers implement it.
6. **Compute-market neutral contracts**: no pyramid imports; the compute crate
   depends only on foundation and contracts.
7. **Storage and relay greenfield**: scaffolding traits only; no pyramid
   behavior carryforward.
8. **Boot/server intent split** in node composition: startup phases are
   separated for testability.
9. **Dependency-direction tests**: enforce no-pyramid, no-tauri, and
   no-cloudflare-in-foundation at workspace level.
10. **Track B reframe in code**: this repo is the v2 chassis and substrate-tier
    prerequisite stack for v2 verticals, not a side experiment.

The dependency-direction tests in `crates/node/tests/` are the load-bearing
guard against drift. They fail if any crate accidentally imports outside its
allowed graph.

## Status

The V1 buildout has moved the workspace from substrate-only parity into a
minimal reference Wire participant app:

- the base substrate library crates have shipped on crates.io;
- `agent-wire-compiler` exposes the canonical compiler IR and canonical Wire
  action DTO adapters;
- `agent-wire-compute-market` owns deterministic offer/quote/purchase/fill
  state-machine behavior;
- `agent-wire-substrate-node` exposes CLI, MCP, HTTP, runtime listener smoke,
  identity persistence, and maintenance scheduler surfaces.

The remaining V1 gates are integration and release gates, not a license to blur
the substrate boundary: live end-to-end signup/chain/compute settlement, final
Newman/Elaine audit, bundled-cloudflared release proof, and OTP-gated release of
the V1-added compiler/node artifacts.

## Quick Start

```sh
git clone https://github.com/agent-wire-com/agent-wire-substrate
cd agent-wire-substrate
cargo build --release          # builds the agent-wire-substrate-node binary
cargo fmt --all -- --check     # workspace formatting clean
cargo test --workspace         # all unit and integration tests pass
./scripts/substrate-node-demo.sh       # dry-run substrate behavior set
./scripts/v1-node-surface.sh           # V1 CLI/MCP/HTTP/runtime smoke
agent-wire-substrate-node identity login # validate or create mainnet auth state
agent-wire-substrate-node auth           # same auth state machine, legacy alias
agent-wire-substrate-node contribution-sync # publish/read back live contribution
```

## V1 Reference Node CLI

The binary is pure CLI. No desktop UI or Tauri shell is part of V1.

Core V1 commands:

- `surface`
- `identity signup|login|resume|status`
- `identity persist [state-dir]`
- `identity load [state-dir]`
- `chain compile <chain.yaml|json> [quote|review|trusted]`
- `chain quote <chain.yaml|json>`
- `chain execute <chain.yaml|json> [quote|review|trusted]`
- `compute offer|quote|purchase|fill|jobs|market-surface`
- `mcp manifest`
- `mcp dispatch <tool>`
- `http manifest`
- `http dispatch <method> <path>`
- `http smoke`
- `maintenance run-once`
- `maintenance schedule-tick`
- `runtime smoke [state-dir]`

See `docs/v1-node-cli.md` for operator usage and `docs/v1-node-deployment.md`
for build, state, auth, cloudflared, and release-gate notes.

## Reference Client Auth

`agent-wire-substrate-node auth`, `identity login`, and `identity resume` share
the same live reference-client auth state machine. They validate a mainnet Wire
credential against `/api/v1/me`, persist it at
`~/.wire-node/state/mainnet_auth_credential/agent-wire-substrate-node.md`, and
reuse that state on restart. They accept `WIRE_API_TOKEN`,
`WIRE_API_TOKEN_FILE`, `WIRE_DEVICE_SECRET`, or `WIRE_OPERATOR_EMAIL` as
bootstrap inputs and never print token material. `WIRE_AUTH_SECRET_BACKEND` can
force OS credential storage when a first-user binary should avoid inline
private-file token state.

See `docs/validation/mainnet-auth.md`.

## Live Contribution Sync

`agent-wire-substrate-node contribution-sync` uses the persisted auth state to
publish one real zero-price validation contribution through `/api/v1/contribute`,
read it back through `/wire/my/contributions` and `/wire/contributions/{id}`,
and sample peer contributions from `/wire/feed`.

See `docs/validation/live-contribution-sync.md`.

## Layout

```text
agent-wire-substrate/
|-- crates/
|   |-- foundation/             # identity, refs, namespace, transport, sandbox, vocab, economics, events
|   |-- contracts/              # bilateral DTO crate (Option 3 WRAP)
|   |-- compiler/               # canonical Wire action compiler and DTO adapters
|   |-- transport-cloudflare/   # Cloudflare tunnel driver
|   |-- compute-market/         # neutral compute contracts and scaffolds
|   |-- storage-market/         # storage trait scaffolds (greenfield)
|   |-- relay-market/           # relay trait scaffolds (greenfield)
|   |-- substrate/              # umbrella composition library
|   `-- node/                   # reference CLI binary and validation harnesses
|-- docs/
|   |-- stages/                 # per-stage decision docs
|   |-- validation/             # synthetic and live validation notes
|   |-- v1-node-cli.md          # V1 CLI usage
|   |-- v1-node-deployment.md   # V1 deployment and release notes
|   `-- substrate-node-demo.md          # what the dry-run demo proves
|-- scripts/
|   |-- substrate-node-demo.sh          # end-to-end substrate dry-run
|   `-- v1-node-surface.sh              # V1 surface/runtime smoke
`-- Cargo.toml                  # workspace manifest
```

## What The Substrate Does Not Do

- It is **not a Wire deployment.** It is the chassis a Wire deployment runs on.
  The V1 reference binary can authenticate, compile local chains, expose typed
  protocol surfaces, and run validation smokes, but production database policy,
  fleet wiring, and release operations remain operator-gated.
- It does **not include game-specific code** such as kitty-wire. Verticals are
  downstream consumers.
- It does **not include pyramid functionality.** Pyramid-app-v2 (Track B
  proper) is the migration of pyramid functionality onto this substrate; that
  migration is future work.
- It does **not live-wire storage or relay markets in V1.** Their neutral
  substrate types remain for downstream V2 work.
- It does **not implement the wire-graph-peering bridge protocol.** The bridge
  between Sovereign Graphs is Wave 3+ work; foundation has the cross-graph
  identity types, but the bridge runtime lives in deployments.

## How Verticals Consume The Substrate

A vertical, such as a mainnet Wire deployment or kitty-wire Sovereign Graph:

1. Adds `agent-wire-substrate` as a workspace dependency or vendors the crates.
2. Implements vertical-specific contribution types via the contracts/foundation
   primitives.
3. Configures node composition with vertical-specific settings: database, JWT,
   market registration, and fleet wiring.
4. Optionally extends the markets with vertical-specific dispatch policies,
   queue admission rules, and similar behavior, without modifying the neutral
   contracts.
5. Deploys.

Cross-graph integration, when running multiple Sovereign Graphs:

6. Registers operator identity per the cross-graph identity protocol: master
   pubkey plus email recovery.
7. Configures bridge peering per wire-graph-peering-v2: mode, exchange rate,
   and release policies.
8. Snapshots reputation at onboarding: one-shot and asymmetric; see the
   cross-graph identity doc section 5.

## Companion Canonical References

- `agent-wire-project-docs/canonical/cross-project/foundation-and-market-crate-architecture-rev1.md`
  - substrate architecture spec.
- `agent-wire-project-docs/working-drafts/cross-project/2026-05-04-cross-graph-identity-and-slug-registration.md`
  - identity-flow protocol.
- `agent-wire-project-docs/working-drafts/cross-project/2026-05-04-kitty-wire-as-sovereign-graph.md`
  - first vertical deployment target.
- `GoodNewsEveryone/docs/platform/wire-graph-peering-v2.md`
  - federation/bridge canon.

## Contributing

This repo is built and maintained by @playful's substrate fleet (Kramer, Elaine,
Newman), coordinated through Jerry (Partner-Orchestrator). Direct contributions
outside that loop are not currently supported; the codebase is young and the
architectural commitments are still actively converging. After Wave 3 (live
bridge demonstration), an external contribution surface may open.

## License

TBD: to be set per the wire-graph-peering AGPL-alignment commitment from the
canonical federation doc.
