# V1 WS1 Foundation Mirror/Wake Slice

This slice adds the foundation-owned mirror and wake primitives that the V1
compiler, cache hydration, and long-running node runtime can share without
recreating node-local shapes.

Shipped:

- `foundation::mirror` with bounded corpus/path/hash types, SHA-256 content
  hashes, cached/remote/local document shapes, mirror links, conflict records,
  and deterministic push/pull/update/hash-match diffing.
- `foundation::wake` with bounded trigger/filter names, cursor/timeout/limit
  request shape, wake batches, wake events, and a `WakeRuntime` trait for
  polling/wait implementations.
- Foundation exports for the new mirror and wake surfaces.

The slice intentionally does not add filesystem walking or an HTTP/SSE client to
foundation. Node/runtime crates own I/O, secrets, and lifecycle; foundation owns
the typed state, diffing, and wait contracts.

Validation:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test -p agent-wire-foundation
cargo test --lib --workspace
cargo test --workspace
cargo build --release
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/substrate-node-demo.sh
test -x target/release/agent-wire-substrate-node
git diff --check
git diff --cached --check
```

The first demo wrapper attempt without `CARGO_HOME` failed because the Codex
sandbox cannot write the default `~/.cargo` registry cache. The rerun above used
the same writable Cargo home as the rest of this validation and passed.

No live Wire event stream, live HTTP call, filesystem sync, deploy, npm publish,
crates.io publish, or L6 runner is part of this slice.
