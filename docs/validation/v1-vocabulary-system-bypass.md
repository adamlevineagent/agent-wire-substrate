# V1 Vocabulary System Bypass Fix

This slice closes Newman WS7 cycle 1 finding `playful/124/25`: the public
`VocabularyKey::system(value)` constructor allowed downstream crates to bypass
reserved primitive and canonical-op collision checks.

Source checked:

- `crates/foundation/src/vocabulary.rs`
- `crates/substrate/src/boot.rs`

Shipped:

- `VocabularyKey::system` is now crate-private to `agent-wire-foundation`.
- The internal system constructor accepts only reserved substrate primitive
  names and still rejects canonical operation names such as `llm`.
- Downstream crates can no longer call `VocabularyKey::system("llm")`; a
  `compile_fail` doc test guards that public API boundary.
- Substrate bootstrap now uses
  `VocabularyEntry::compute_primitive_entry(vocabulary, definition_ref)`, a
  fixed foundation-owned constructor for the legitimate `compute-market`
  bootstrap entry rather than an arbitrary string bypass.
- The substrate composition test asserts the fixed bootstrap entry still
  resolves to `compute-market`.

Validation:

```sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo fmt --all -- --check
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-foundation vocabulary
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-substrate
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/substrate-node-demo.sh
test -x target/release/agent-wire-substrate-node
rg -n "VocabularyKey::system|pub\\(crate\\) fn system|compute_primitive_entry" crates docs -g '*.rs' -g '*.md'
git diff --check
git diff --cached --check
```

No runtime HTTP call, live DB call, deploy, npm publish, crates.io publish, or
L6 calendar runner is part of this fix.
