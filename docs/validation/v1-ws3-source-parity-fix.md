# V1 WS3 Source-Parity Fix

Elaine's serial audit found that the WS3 compiler crate exposed Rust-native
serde shapes where canonical GoodNewsEveryone action JSON expects camelCase
fields and `tool` strings.

## Finding

The internal Rust structs are typed for local execution:

- `WireActionStep` stores `wire: Option<WirePrimitive>` and
  `task: Option<TaskPrimitive>`.
- `WireActionDefinition` stores `contribution_type` and `action_kind`.
- `WireCompiledPlan` stores local execution metadata such as `quote_receipt`,
  `invocation_mode`, and per-step compiled operations.

GoodNewsEveryone's canonical contribution definition shape is different:

- `schemaVersion`
- `actionType: "chain"`
- camelCase step fields (`outputSchema`, `modelTier`, `forEach`, `actionId`,
  `waitFor`, `gameType`, `entryFee`)
- `tool` strings for Wire/Task steps
- `compiledPlan.totalSteps/maxCost/operationsUsed/resolvedActions/compiledAt`

## Fix

- Added explicit canonical DTO/adaptor types in `agent-wire-compiler`:
  - `CanonicalWireActionDefinition`
  - `CanonicalWireActionStep`
  - `CanonicalWireCompiledPlan`
  - `CanonicalWireActionPermissions`
- Kept the richer typed Rust `WireCompiledPlan` internal.
- Converted canonical `tool` strings to typed `WirePrimitive` /
  `TaskPrimitive` by scanning foundation-owned sealed registries, not by
  runtime string-arm dispatch.
- Updated the node chain loader so `chain compile <chain.yaml|json>` accepts
  canonical Wire action JSON/YAML and converts into the internal typed compiler
  plan.

## Validation

- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-compiler canonical`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate-node cli_loader_accepts_canonical_wire_json_shape`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo fmt --all -- --check`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-compiler`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate-node v1`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home scripts/v1-node-surface.sh`
- `CARGO_HOME=/private/tmp/codex-kramer-cargo-home scripts/substrate-node-demo.sh`
- `test -x target/release/agent-wire-substrate-node`
- `git diff --check`
- `git diff --cached --check`
- sealed-registry string-dispatch probe: no hits
