# Stage 1 Contracts And Foundation Decision

Mission: `playful/122/6`

The locked contracts decision is Option 3 with the WRAP verb:

- `agent-wire-contracts` remains a separate crate for bilateral wire DTOs and serialized protocol surfaces.
- `agent-wire-foundation` defines local runtime types.
- Foundation depends on contracts and implements explicit `From<ContractDto>` / `From<FoundationType>` conversions at the boundary.
- Foundation does not re-export contracts types. Consumers choose whether they are operating on protocol DTOs or local runtime types.
- Contract handoff uses `ContractWrap<T>` with `verb = "wrap"` so wrapped wire DTOs are distinguishable from local foundation values.

The first foundation skeleton intentionally avoids market, node-app, Tauri, Cloudflare-driver, and vertical imports. Those dependency directions are acceptance criteria, enforced by the foundation crate tests.
