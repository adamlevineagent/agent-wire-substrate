# Agent Wire Substrate Node 2.0 Parity Demo

The Stage 10 parity demo is intentionally substrate-tier only. It proves that the new
`agent-wire-substrate-node` binary can compose every Wave 1 crate into a single local node runtime without
depending on pyramid-app code.

Run:

```sh
scripts/substrate-node-demo.sh
```

The demo constructs:

- mainnet authentication material through foundation identity primitives
- local config, persistence, sandbox, and opt-in policy
- operator HTTP, MCP, IPC, and REST route surface
- Cloudflare tunnel session through the foundation transport trait
- compute-market participation on both sides: provider offer and requester job contract
- storage-market offer for pin and retrieval wiring
- relay-market offer for tunnel path lease wiring
- vocabulary entry handling through foundation vocabulary primitives
- wake-up trigger filter for unread messages, task moves, task assignments, and contribution events

The script is a dry-run. It does not contact mainnet, open a live tunnel, execute a model, deploy,
publish an npm package, or run live smoke. Those remain operator-gated follow-up work after the
substrate composition surface is accepted.
