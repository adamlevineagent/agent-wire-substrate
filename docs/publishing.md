# Publishing agent-wire-substrate crates

The workspace is split into individually publishable crates plus a thin CLI crate:

1. `agent-wire-contracts`
2. `agent-wire-foundation`
3. `agent-wire-compute-market`
4. `agent-wire-storage-market`
5. `agent-wire-relay-market`
6. `agent-wire-transport-cloudflare`
7. `agent-wire-substrate`

`agent-wire-substrate-node` is the binary shell around the publishable umbrella library.

## Dry-run status

`agent-wire-contracts` passed `cargo publish --dry-run -p agent-wire-contracts --allow-dirty`.

The dependent crates package their manifests, then stop during local publish preparation because Cargo validates dependency versions against the crates.io index. Until the prior internal crates are actually published, the public index has no matching packages:

- `agent-wire-foundation`: missing `agent-wire-contracts`
- `agent-wire-compute-market`: missing `agent-wire-contracts`
- `agent-wire-storage-market`: missing `agent-wire-foundation`
- `agent-wire-relay-market`: missing `agent-wire-foundation`
- `agent-wire-transport-cloudflare`: missing `agent-wire-foundation`
- `agent-wire-substrate`: missing `agent-wire-compute-market`

After each real publish lands in the crates.io index, rerun the next crate's dry-run before publishing it.

## OTP-gated publish sequence

Run from the workspace root after the extraction commit is on `main`:

```bash
cargo publish -p agent-wire-contracts
cargo publish --dry-run -p agent-wire-foundation
cargo publish -p agent-wire-foundation
cargo publish --dry-run -p agent-wire-compute-market
cargo publish -p agent-wire-compute-market
cargo publish --dry-run -p agent-wire-storage-market
cargo publish -p agent-wire-storage-market
cargo publish --dry-run -p agent-wire-relay-market
cargo publish -p agent-wire-relay-market
cargo publish --dry-run -p agent-wire-transport-cloudflare
cargo publish -p agent-wire-transport-cloudflare
cargo publish --dry-run -p agent-wire-substrate
cargo publish -p agent-wire-substrate
```

The actual publish commands remain Adam/`@playful` OTP-gated.
