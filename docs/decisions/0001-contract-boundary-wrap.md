# Decision 0001: Contract Boundary Uses WRAP

Status: accepted for Wave 1 Stage 1

## Decision

The `agent-wire-contracts` relationship is Option 3 with a WRAP verb.

Foundation does not re-export `agent-wire-contracts`. Any crate that receives or
emits contract DTOs must wrap them at the boundary and convert them into local
substrate runtime types with explicit `From` or `TryFrom` implementations.

## Why

Re-exporting the contracts crate would erase the seam between bilateral protocol
DTOs and local substrate behavior. The seam needs to stay visible in code review,
search, and dependency audits.

The WRAP verb gives that seam a greppable shape:

- `wrap_*` when a contract DTO enters a local substrate boundary
- `unwrap_*` only where a boundary intentionally emits a contract DTO
- local runtime types stay distinct from protocol DTOs

## Guardrail

Foundation owns substrate primitives only. It may define generic wrapper
mechanics, but it must not depend on or re-export `agent-wire-contracts`.
