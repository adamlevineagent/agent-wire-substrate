# Stage 8 - Storage Market Scaffold

Stage 8 adds the greenfield storage-market contract surface without importing the old market
heuristics. The crate defines only minimal trait and data shapes for:

- storage offers
- cross-graph capacity allocation
- pin commitments
- retrieval requests and receipts
- replication factor
- retention policy
- settlement intent

The acceptance target is scaffolding parity for later node composition. Implementations,
provider selection, pricing heuristics, and replication engines remain out of scope.
