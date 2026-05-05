# Publishing agent-wire-substrate crates

The workspace is split into eight library crates plus a reference-node binary
crate. The original substrate extraction set has already shipped on crates.io;
the V1-added compiler and node release remain OTP-gated.

1. `agent-wire-contracts`
2. `agent-wire-foundation`
3. `agent-wire-compute-market`
4. `agent-wire-storage-market`
5. `agent-wire-relay-market`
6. `agent-wire-transport-cloudflare`
7. `agent-wire-compiler`
8. `agent-wire-substrate`

`agent-wire-substrate-node` is the reference binary shell around the publishable
umbrella library. Treat its release as a binary/runtime release, not as a
replacement for the library-crate publish order.

## Registry status

The base substrate crates are visible on crates.io:

- `agent-wire-contracts`
- `agent-wire-foundation`
- `agent-wire-compute-market`
- `agent-wire-storage-market`
- `agent-wire-relay-market`
- `agent-wire-transport-cloudflare`
- `agent-wire-substrate`

As of the V1 docs pass, `cargo search agent-wire-compiler` and
`cargo search agent-wire-substrate-node` do not return published packages.

## Dry-run status

Before any V1 publish, rerun dry-runs from the clean release commit. Cargo
validates dependency versions against the crates.io index, so V1 crates that
depend on newly added internal crates may need the preceding crate to land in
the public index before their dry-run can pass.

## OTP-gated publish sequence

Run from the workspace root after the release commit is on `main`:

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
cargo publish --dry-run -p agent-wire-compiler
cargo publish -p agent-wire-compiler
cargo publish --dry-run -p agent-wire-substrate
cargo publish -p agent-wire-substrate
```

The actual publish commands remain Adam/`@playful` OTP-gated. If the base
substrate crates are already at the desired version, skip republishing them and
start from the first V1 crate whose manifest version is not yet on crates.io.
Do not run `cargo publish` from worker sessions.
