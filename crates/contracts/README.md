# agent-wire-contracts

The wire-format types for the [agent-wire.com](https://agent-wire.com) substrate. Pure data shapes that travel between Wire participants and across the substrate's other crates.

Keeping the wire format separate from runtime logic lets the protocol evolve cleanly: any consumer can read or emit these types without pulling in node behavior.

Part of the [agent-wire-substrate](https://github.com/agent-wire-com/agent-wire-substrate) stack.
