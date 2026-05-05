# V1 WS-Compute State Machine Slice

This slice moves the compute-market core from contract shapes into a deterministic
substrate-owned state machine, while keeping runtime I/O and provider execution
behind typed seams.

Source checked:

- `/Users/adamlevine/AI Project Files/agent-wire-node/src-tauri/src/compute_market.rs`
- `/Users/adamlevine/AI Project Files/agent-wire-node/src-tauri/src/compute_queue.rs`
- `/Users/adamlevine/AI Project Files/GoodNewsEveryone/docs/architecture/wire-node-compute-market-contract.md`

Shipped:

- `ComputeMarketStateMachine` with deterministic offer publication, quote
  planning, purchase reservation, fill dispatch, queue mirror, and market
  surface behavior.
- Quote, purchase, and fill idempotency keyed by foundation-owned
  `QuoteReceipt`, `IdempotencyKey`, and `FillKey` primitives.
- Budget, quote TTL, single-redeem quote, reservation/offer, and queue-cap
  rejection paths.
- Neutral compute envelope/output and `ComputeExecutionAdapter`, so model
  invocation adapters can execute without pyramid, desktop, or server-local
  types crossing into the market core.
- Job lifecycle state for dispatched, executing, completed, failed, and settled
  jobs, including queue-slot release on terminal execution outcomes.

Boundary:

- `agent-wire-compute-market` owns deterministic market state and typed
  transitions.
- Node/runtime code owns persistence, HTTP/MCP/CLI bindings, secrets,
  cloudflared lifecycle, provider process supervision, and actual inference I/O.

Validation:

```sh
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo fmt --all -- --check
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo check --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test -p agent-wire-compute-market
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --lib --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo test --workspace
CARGO_HOME=/private/tmp/codex-kramer-cargo-home cargo build --release
CARGO_HOME=/private/tmp/codex-kramer-cargo-home ./scripts/substrate-node-demo.sh
test -x target/release/agent-wire-substrate-node
git diff --check
git diff --cached --check
```

No runtime HTTP client, filesystem persistence, provider process execution, live
DB call, deploy, npm publish, crates.io publish, or L6 calendar runner is part
of this slice.
