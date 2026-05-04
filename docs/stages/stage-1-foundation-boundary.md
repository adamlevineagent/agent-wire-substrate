# Stage 1: Foundation Boundary Skeleton

Wave 1 starts with a small, enforceable foundation surface.

## In Scope

- Identity primitives for master public keys, signed handle claims, private alias
  mappings, private graph registrations, and master key rotations.
- Cross-graph references and handle paths.
- Transport endpoint primitives, including `TunnelUrl`, before any specific
  tunnel driver implementation.
- Integer credit amounts, event envelopes, event cursors, namespace identifiers,
  vocabulary mechanism primitives, and sandbox capability shapes.
- Workspace crate skeletons for transport, compute, storage, relay, and node
  composition.
- Tests that keep foundation from importing vertical, driver, or node crates.

## Out of Scope

- Static pyramid schema mappings.
- Compute-market behavior extraction.
- Storage-market and relay-market implementation.
- Node v2 composition and vertical API movement.
- Production deploys, live smoke, npm publish, or shared-checkout writes.
