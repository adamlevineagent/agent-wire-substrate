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
the original Wave 2 validation checks plus replay-hardening checks added after the Newman V1
adversarial pass:

| Test | Status | What it proves |
|---|---|---|
| Provider registers, requester queries roster, sees provider | PASS | Foundation roster and identity primitives |
| Requester publishes a `ComputeJobEnvelope` contribution | PASS | Compute-market neutral contracts accept real envelopes |
| Provider sees envelope via subscription, claims it | PASS | `EventSink`, `DispatchPolicy`, and `QueueAdmission` cooperate end-to-end |
| Duplicate job through rotated tunnel cannot double-claim | PASS | Job claims are idempotent across tunnel rotation and replay |
| Provider returns synthetic completion / echo response | PASS | `ChronicleSink` and `ExecutionAdapter` trace lifecycle |
| Duplicate compute completion is rejected | PASS | Compute completions are single-shot per `job_ref` |
| Requester reads completion, both sides settle credits | PASS | Foundation economics primitives settle both sides |
| Storage-market write/read of a 1MB blob | PASS | Storage-market trait scaffold round-trips the blob |
| Relay-market subscribe/publish | PASS | Relay-market scaffold leases a path and ferries a message |
| Cloudflare tunnel rotation triggered manually mid-flight | PASS | Transport rotation preserves in-flight contributions |

## Property Coverage

| Test | Property tested | Property NOT tested |
|---|---|---|
| Provider registers, requester queries roster, sees provider | A provider offer inserted into the synthetic graph is visible to requester-side roster lookup. | Live provider discovery, production reputation ranking, or multi-provider market depth. |
| Requester publishes a `ComputeJobEnvelope` contribution | The composed compute contract can be published as a graph event with budget and dispatch constraints intact. | Live contribution storage, real Wire API publication, or live requester authentication. |
| Provider sees envelope via subscription, claims it | The event sink, dispatch policy, and queue-admission shape admit exactly the first claim. | Real concurrent subscribers, durable claim locks, or database transaction isolation. |
| Duplicate job through rotated tunnel cannot double-claim | A replayed claim after synthetic tunnel rotation sees the original job but cannot insert a second claim for the same `job_ref`. | Production distributed-lock contention, hostile tunnel provider behavior, or cross-process crash recovery. |
| Provider returns synthetic completion / echo response | A claimed job can be completed through the deterministic execution adapter and recorded in the chronicle sink. | Live model inference, provider billing disputes, or non-deterministic output validation. |
| Duplicate compute completion is rejected | A completed `job_ref` cannot record a second completion receipt. | Production idempotency persistence across process restarts or database uniqueness constraints. |
| Requester reads completion, both sides settle credits | The requester can read the synthetic completion and settle within the declared max price. | Live `wire_settlements` writes, real escrow clearing, or negative-balance handling. |
| Storage-market write/read of a 1MB blob | The storage trait scaffold can pin and retrieve the existing decimal 1MB capacity target. | Live blob storage, replication durability, or provider slashing. |
| Relay-market subscribe/publish | The relay scaffold can lease one direct path and deliver one subscribed message. | Multi-hop privacy routing, packet loss, or live relay peering. |
| Cloudflare tunnel rotation triggered manually mid-flight | A second synthetic tunnel rotation preserves visibility of an in-flight contribution. | Live Cloudflare tunnel operations, DNS propagation, or long-lived socket migration. |

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
