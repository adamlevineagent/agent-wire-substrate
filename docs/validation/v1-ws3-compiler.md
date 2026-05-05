# V1 WS3 Compiler Slice

This slice starts the canonical Wire compiler port as a new sibling crate,
`agent-wire-compiler`, consuming the foundation sealed-registry op surface
instead of introducing compiler-local free-string dispatch.

Source checked:

- `playful/124/15`
- `playful/123/91`
- `playful/123/93`
- `playful/123/106`
- `/Users/adamlevine/AI Project Files/agent-wire-project-docs/working-drafts/cross-project/2026-05-05-substrate-node-v1-port-plan.md`
- `/Users/adamlevine/AI Project Files/agent-wire-project-docs/working-drafts/cross-project/2026-05-05-canonical-wire-op-manifest.md`

Shipped:

- Workspace crate `agent-wire-compiler`.
- Typed action IR for contribution type, action kind, action permissions, action
  steps, step modifiers, invocation modes, compiled steps, and compiled plans.
- `WireCompiler` with action-name/step validation, typed operation validation,
  permission checks, nested action resolution, cost estimation, quote-receipt
  generation, and explicit V1 disposition for game stubs.
- V1 compiler op manifest composed from
  `agent_wire_foundation::vocabulary::canonical_ops`.
- Foundation sealed registry expansion for `wire.retract` and task-board
  primitives: `task.create`, `task.claim`, and `task.complete`.
- Substrate umbrella composition now exposes the compiler manifest through
  `compose_substrate_node`.

Boundary:

- Foundation owns canonical op identity and collision rejection.
- Compiler owns action IR, validation, quote planning, and compile-time
  disposition.
- Node/runtime will own CLI/HTTP/MCP binding and actual execution I/O.
- Game compiles as an explicit `OutOfV1Scope` disposition.

Validation:

```sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo fmt --all -- --check
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-compiler
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-foundation vocabulary
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/substrate-node-demo.sh
test -x target/release/agent-wire-substrate-node
rg -n "match\\s+op_name|match\\s+operation_name|\\\"llm\\\"\\s*=>|\\\"wire\\\"\\s*=>|\\\"task\\\"\\s*=>|\\\"game\\\"\\s*=>" crates/compiler crates/foundation crates/substrate crates/node crates/compute-market
git diff --check
git diff --cached --check
```

No live Wire call, runtime HTTP binding, CLI command surface, deploy, npm
publish, crates.io publish, live DB call, or L6 calendar runner is part of this
slice.
