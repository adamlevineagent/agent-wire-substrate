# V1 Hard-Prereq Foundation Slice

This slice starts the `agent-wire-substrate-node` V1 build mission by landing the
cross-cutting primitives that later workstreams must consume instead of
recreating locally:

- Foundation economics now owns bounded `IdempotencyKey`, `FillKey`,
  `QuoteReceipt`, `SettlementCommit`, and `SettlementSettled` types.
- Serde for those economic primitives routes through constructors, so invalid
  keys or over-price settlements cannot bypass the Rust boundary.
- Compute-market quote, purchase, and fill request shapes consume the foundation
  idempotency/fill primitives directly.
- Sandbox `ResourceBudget` now includes stack, heap, and recursion ceilings, and
  serde preserves constructor validation while accepting legacy budget shapes
  with safe defaults.
- `foundation::vocabulary::canonical_ops` is the sealed registry for compiler,
  LLM, Wire primitive, step modifier, invocation-mode, and maintenance task
  names. User vocabulary keys now reject foundation-registered names.

Validation:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test -p agent-wire-foundation
cargo test -p agent-wire-compute-market
cargo test --lib --workspace
```

No live Wire traffic, live database write, deploy, npm publish, or crates.io
publish is part of this slice.
