# Stage 9 - Relay Market Scaffold

Stage 9 adds the greenfield relay-market contract surface. It composes foundation endpoint
and tunnel primitives but does not implement routing, discovery, or payment execution.

The crate defines minimal trait and data shapes for:

- relay offers
- path lease requests
- hop capabilities
- privacy tiers
- rotation policy
- relay path leases
- per-hop settlement

The acceptance target is scaffolding parity for later node composition. Full path selection,
relay rotation engines, privacy enforcement, and settlement execution remain out of scope.
