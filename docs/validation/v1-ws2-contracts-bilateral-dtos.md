# V1 WS2 Contracts Bilateral DTO Slice

This slice fills the `agent-wire-contracts` bilateral compute DTO gap identified
in `playful/123/93`.

Source checked:

- `/Users/adamlevine/AI Project Files/GoodNewsEveryone/packages/agent-wire-contracts/rust/src/lib.rs`

Shipped:

- Error envelope and detail DTOs for compute route failures.
- Quote and purchase DTOs, including latency preference, purchase triggers,
  queue position, and matched queue depth.
- Retry, compute failure, and delivery failure enums plus their advertised
  allow-list constants.
- Market surface response, stream event, history, provider, offer, depth, and
  catalog DTOs.
- Chronicle `ComputeEventType`, `COMPUTE_SQLSTATE`, and
  `COMPUTE_ERROR_EVENT_TYPES`.
- Sealed `WireDto` coverage for every new serializable contract shape, keeping
  protocol DTO implementation ownership inside `agent-wire-contracts`.

Validation:

```sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo fmt --all -- --check
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-contracts
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/substrate-node-demo.sh
comm -23 <(rg -o "pub (struct|enum|const) [A-Za-z0-9_]+" "/Users/adamlevine/AI Project Files/GoodNewsEveryone/packages/agent-wire-contracts/rust/src/lib.rs" | awk '{print $3}' | sort -u) <(rg -o "pub (struct|enum|const) [A-Za-z0-9_]+" crates/contracts/src/lib.rs | awk '{print $3}' | sort -u)
```

The symbol parity check produced no missing source public structs, enums, or
constants.

No runtime state-machine logic, live HTTP call, deploy, npm publish,
crates.io publish, filesystem sync, or L6 runner is part of this slice.
