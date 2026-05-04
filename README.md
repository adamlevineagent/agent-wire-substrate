# agent-wire-substrate

Greenfield substrate-tier workspace for Wire Node 2.0.

Stage 1 records the locked contracts boundary:

- `agent-wire-contracts` owns bilateral wire DTOs.
- `agent-wire-foundation` owns local runtime types and explicit conversions.
- Foundation depends on contracts but does not re-export contracts types.
- Cross-boundary handoff uses the `wrap` contract envelope verb.

Stage 2 begins the foundation skeleton:

- client-side identity primitives
- handle-path and cross-graph reference types
- endpoint and callback URL wrappers, including `TunnelUrl`
- `CreditAmount`
- event envelope and cursor primitives
- `NamespaceId`
- dependency-direction tests that forbid foundation imports from verticals, markets, node app, Tauri, or Cloudflare driver code
