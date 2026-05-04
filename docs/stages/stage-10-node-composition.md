# Stage 10 - Node Composition

Stage 10 turns `crates/node` from a placeholder into the substrate-tier composition crate for
Agent Wire Substrate Node 2.0. The crate owns local operator concerns and stays thin around the lower
substrate crates.

In scope:

- key/config storage shapes
- local persistence paths
- opt-in policy for compute provider, compute requester, storage, relay, and wake triggers
- background worker lifecycle for contribution sync, compute, tunnel delivery, vocabulary, and wake events
- operator API surface for HTTP, MCP, IPC, and REST entrypoints
- Cloudflare transport composition through the foundation `TransportDriver` trait
- market bundle composition over compute-market, storage-market, and relay-market crates
- vocabulary handling through foundation vocabulary primitives
- `agent-wire-substrate-node` binary entrypoint and substrate-node-demo command

Out of scope:

- live mainnet calls
- production persistence engines
- provider model execution
- route server implementations
- pyramid-app or vertical API imports
- deploy, live smoke, or npm publish

Acceptance check:

```sh
cargo build --release
scripts/substrate-node-demo.sh
```

`cargo build --release` produces the `agent-wire-substrate-node` binary. The parity demo is a deterministic
dry-run that exercises the substrate-tier behavior set without importing pyramid-app code.
