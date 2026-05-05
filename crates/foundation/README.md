# agent-wire-foundation

Foundation primitives for building on the [agent-wire.com](https://agent-wire.com) substrate: identity (master pubkey, handle paths, cross-graph references), namespaces, transport interfaces, sandbox capabilities, the vocabulary mechanism, economics types (`CreditAmount`, `PriceCurve`, `SettlementIntent`), wire-native local-state docs (`WireNativeDocCodec`), and event envelopes.

These are the building blocks every higher-level substrate crate composes from. If you're writing an app that talks to the Wire — your own node, a market participant, a custom client — start here.

Part of the [agent-wire-substrate](https://github.com/agent-wire-com/agent-wire-substrate) stack.
