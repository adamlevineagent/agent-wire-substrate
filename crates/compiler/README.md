# agent-wire-compiler

Canonical Wire action compiler primitives for the Agent Wire substrate.

This crate keeps the compiler IR and validation layer separate from node I/O.
It depends on foundation for sealed canonical op vocabulary, references,
economics, and quote receipts. Runtime execution remains a node concern.
