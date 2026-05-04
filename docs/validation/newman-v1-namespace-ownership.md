# Newman V1 Namespace-Ownership Hardening

This pass closes the substrate-side Newman V1 structural findings by moving enforcement into the
foundation and synthetic harnesses instead of relying on naming conventions.

Scope:

- Foundation dependency direction now has a `cargo metadata` transitive dependency guard in
  addition to the source-token guard.
- Contract DTOs are sealed `WireDto` shapes, and foundation conversions go through
  `WrappedContract<T>` instead of public raw DTO unwrap helpers.
- Reputation snapshots carry master signatures, import verifies statement-bound bytes, and kitty
  rejects duplicate snapshot imports.
- Layer 3 compute claim and completion ledgers reject replayed claim/completion attempts.
- Vocabulary entries use bounded constructors, typed `CrossGraphRef` definition anchors, and a
  reserved primitive-name registry with explicit system construction for built-ins.
- Sandbox capability extensions use bounded, validated `ExtensionCapability` names; policies bind
  immutable grants and budgets; `BudgetAccountant` defines the interpreter-side invariant hook.

Validation:

```sh
cargo fmt --all -- --check
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/substrate-node-demo.sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/layer3-single-graph-synthetic.sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/layer4-two-graph-bridged-synthetic.sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release
git diff --check
```

No deploy, live smoke, live database write, live LLM call, or npm publish is part of this pass.
