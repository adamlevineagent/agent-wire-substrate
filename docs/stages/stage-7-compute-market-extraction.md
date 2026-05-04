# Stage 7 - Compute Market Extraction Scaffold

Stage 7 extracts the compute-market contract surface from the existing GoodNewsEveryone
compute marketplace into a greenfield substrate crate. It keeps the durable market nouns
and boundaries while leaving route handlers, database RPCs, matcher heuristics, and provider
transport execution out of scope.

The crate defines typed serde shapes for:

- compute offers, provider nodes, local versus bridge providers, price curves, reservation fees,
  queue discount curves, and max queue depth
- quote requests and signed quote results with explicit price breakdowns
- purchase reservations and immediate or deferred purchase triggers
- queue mirror snapshots that expose only market-safe depth and visibility fields
- fill and market dispatch requests, scoped credentials, dispatch outcomes, and delivery receipts
- market-surface snapshots for read-side composition
- retry intent and structured compute failure codes

The acceptance target is extraction parity for later node composition. Stage 7 does not port
the production SQL functions, quote JWT implementation, queue mirror cache invalidation, route
auth, model matching heuristics, or provider tunnel dispatch implementation.
