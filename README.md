# agent-wire-substrate

The substrate-tier of the Wire: foundation crate, transport drivers, and the
three markets (compute, storage, relay), packaged as a Cargo workspace and
composed by the `agent-wire-substrate-node` v2 binary.

This is the chassis. Verticals such as mainnet Wire, kitty-wire, and future
Sovereign Graphs deploy on top of it. Pyramid-app-v2 (Track B) eventually
migrates onto it.

## What This Repo Is

`agent-wire-substrate` is a Rust Cargo workspace with seven crates:

| Crate | Role |
| --- | --- |
| `foundation` | Identity and cross-graph types, refs, namespace, transport trait, sandbox, vocabulary mechanism, economics primitives, event envelope, contracts boundary, dependency guard |
| `contracts` | Bilateral DTO crate; `From<ContractDto>` conversions only, with no inherent methods (Option 3 WRAP) |
| `transport-cloudflare` | Cloudflare driver implementing the foundation transport trait (P2P tunnel, default 12h rotation) |
| `compute-market` | Neutral compute-market contracts (`ComputeJobEnvelope`, `ModelInvocation`, `ExecutionAdapter`, `DeliveryPolicy`, `EventSink`, `ChronicleSink`, `QueueAdmission`, `DispatchPolicy`, `ComputeJobContract`) plus provider/requester scaffolds |
| `storage-market` | Storage-market trait scaffold (greenfield) |
| `relay-market` | Relay-market trait scaffold (greenfield) |
| `node` | Composes foundation, contracts, transport-cloudflare, and all three markets; produces the `agent-wire-substrate-node` binary; ships `scripts/substrate-node-demo.sh` |

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

Wave 1 (substrate-tier rebuild) closed on commit chain:

`bbf6af2 -> 79de7ba -> eae8914 -> 9974d0c -> 82f0f67e -> d4773723`

That chain covers Stages 1, 2, 3, 4, 5, 7, 8, 9, and 10; Stage 6 collapsed as a
numbering gap. Mission `/122/6` (substrate-tier rebuild for pyramid-less Node
2.0 parity) met acceptance criteria at compile/test level.

Mainnet authentication, live syncs, and live-market roundtrips are not exercised
in Wave 1. Architectural parity is at compile/test level only. Live-deploy
validation is Wave 2 work.

## Quick Start

```sh
git clone https://github.com/adamlevineagent/agent-wire-substrate
cd agent-wire-substrate
cargo build --release          # builds the agent-wire-substrate-node binary
cargo fmt --check              # workspace formatting clean
cargo test --workspace         # all unit and integration tests pass
./scripts/substrate-node-demo.sh       # dry-run substrate behavior set
agent-wire-substrate-node auth # validate persisted mainnet auth state
```

## Reference Client Auth

`agent-wire-substrate-node auth` is the first live reference-client surface. It
validates a mainnet Wire credential against `/api/v1/me`, persists it at
`~/.wire-node/state/agent-wire-substrate-node-auth.json`, and reuses that state
on restart. It accepts `WIRE_API_TOKEN`, `WIRE_API_TOKEN_FILE`,
`WIRE_DEVICE_SECRET`, or `WIRE_OPERATOR_EMAIL` as bootstrap inputs and never
prints token material.

See `docs/validation/mainnet-auth.md`.

## Layout

```text
agent-wire-substrate/
|-- crates/
|   |-- foundation/             # identity, refs, namespace, transport, sandbox, vocab, economics, events
|   |-- contracts/              # bilateral DTO crate (Option 3 WRAP)
|   |-- transport-cloudflare/   # Cloudflare tunnel driver
|   |-- compute-market/         # neutral compute contracts and scaffolds
|   |-- storage-market/         # storage trait scaffolds (greenfield)
|   |-- relay-market/           # relay trait scaffolds (greenfield)
|   `-- node/                   # composition crate, binary, parity demo
|-- docs/
|   |-- stages/                 # per-stage decision docs
|   |-- validation/             # synthetic and live validation notes
|   `-- substrate-node-demo.md          # what the dry-run demo proves
|-- scripts/
|   `-- substrate-node-demo.sh          # end-to-end substrate dry-run
`-- Cargo.toml                  # workspace manifest
```

## What The Substrate Does Not Do

- It is **not a Wire deployment.** It is the chassis a Wire deployment runs on.
  Without database, JWT-issuer config, market provider registration, and fleet
  wiring, it does nothing useful at runtime by design.
- It does **not include game-specific code** such as kitty-wire. Verticals are
  downstream consumers.
- It does **not include pyramid functionality.** Pyramid-app-v2 (Track B
  proper) is the migration of pyramid functionality onto this substrate; that
  migration is future work.
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
