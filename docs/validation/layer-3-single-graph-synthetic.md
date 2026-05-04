# Layer 3 Single-Graph Synthetic Validation

Wave 2 Layer 3 adds a deterministic single-graph synthetic validation harness for
`agent-wire-substrate`. It runs inside the `agent-wire-substrate-node` crate and exercises the substrate-tier
contracts, traits, and node composition without live LLM calls, live database access, deploy, live
smoke, or npm publish.

Run:

```sh
scripts/layer3-single-graph-synthetic.sh
```

The harness composes the Stage 10 node runtime, creates a synthetic in-memory graph, and executes
the eight checks from the Wave 2 validation plan:

| Test | Status | What it proves |
|---|---|---|
| Provider registers, requester queries roster, sees provider | PASS | Foundation roster and identity primitives |
| Requester publishes a `ComputeJobEnvelope` contribution | PASS | Compute-market neutral contracts accept real envelopes |
| Provider sees envelope via subscription, claims it | PASS | `EventSink`, `DispatchPolicy`, and `QueueAdmission` cooperate end-to-end |
| Provider returns synthetic completion / echo response | PASS | `ChronicleSink` and `ExecutionAdapter` trace lifecycle |
| Requester reads completion, both sides settle credits | PASS | Foundation economics primitives settle both sides |
| Storage-market write/read of a 1MB blob | PASS | Storage-market trait scaffold round-trips the blob |
| Relay-market subscribe/publish | PASS | Relay-market scaffold leases a path and ferries a message |
| Cloudflare tunnel rotation triggered manually mid-flight | PASS | Transport rotation preserves in-flight contributions |

Scope notes:

- The database is modeled as a transient in-memory graph fixture. Layer 3 does not introduce a
  production persistence engine.
- The 1MB storage case uses the existing decimal capacity from Stage 10 (`1,000,000` bytes).
- The Cloudflare rotation case uses the driver trait with static synthetic tunnel URLs; it does not
  open a live tunnel.
- The compute completion is an echo adapter, not a live model invocation.

Acceptance:

```sh
cargo test --workspace
scripts/layer3-single-graph-synthetic.sh
```

Both commands should remain green before advancing to Layer 4 two-graph bridged synthetic
validation.
