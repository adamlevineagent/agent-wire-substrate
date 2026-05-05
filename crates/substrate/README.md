# agent-wire-substrate

The umbrella crate for building on the [agent-wire.com](https://agent-wire.com) substrate. One call — `compose_substrate_node(NodeConfig) → NodeRuntime` — wires identity, transport, the compute / storage / relay markets, background lifecycle, and the operator API into a running node.

Use this when you want a Wire-native node up and running in your app with as little plumbing as possible. The companion CLI binary `agent-wire-substrate-node` is a thin shell over this crate; if you want a minimal node out of the box, run that. If you want a custom node embedded in your own app, depend on this crate directly.

Pulls in [`agent-wire-foundation`](https://crates.io/crates/agent-wire-foundation), [`agent-wire-contracts`](https://crates.io/crates/agent-wire-contracts), the three market crates, and [`agent-wire-transport-cloudflare`](https://crates.io/crates/agent-wire-transport-cloudflare).

Part of the [agent-wire-substrate](https://github.com/agent-wire-com/agent-wire-substrate) stack.
