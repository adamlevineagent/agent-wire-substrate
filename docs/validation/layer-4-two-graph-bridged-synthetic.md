# Layer 4 Two-Graph Bridged Synthetic Validation

Wave 2 Layer 4 adds a deterministic two-graph harness for `agent-wire-substrate`. It models one
mainnet-shaped graph and one kitty-shaped Sovereign Graph in memory, then exercises the cross-graph
identity and bridge behaviors from the Wave 2 validation plan and the cross-graph identity
working-draft.

Run:

```sh
scripts/layer4-two-graph-bridged-synthetic.sh
```

The harness intentionally stays substrate-tier only. It does not open a live bridge, live tunnel,
live database, live LLM provider, live smoke, deploy, or npm publish.

| Test | Status | What it proves |
|---|---|---|
| Identity claim with master signature works on both graphs | PASS | `HandleClaim`, `PrivateAliasMapping`, `PrivateGraphRegistration`, master public key portability, and mid-slug parsing operate together |
| Reputation snapshot at kitty onboarding captures mainnet score | PASS | Kitty imports a one-shot signed mainnet reputation snapshot |
| Reputation snapshot import is one-shot | PASS | Duplicate imports return `SnapshotAlreadyImported` and tampered snapshots fail statement-bound signature verification |
| Mainnet reputation evolution does not propagate post-snapshot | PASS | The reputation firewall is one-shot and asymmetric |
| Kitty contribution with `release_to_mainnet=true` surfaces on mainnet | PASS | Bridge release policy carries a mid-slug contribution into mainnet |
| Credit transferred kitty to mainnet incurs bridge tax | PASS | Sovereign-mode 2 percent bridge friction applies at par |
| Compute job dispatched mainnet-side fulfills via kitty-side provider | PASS | Cross-graph compute-market routing can reach a kitty provider |
| Provider reputation in mainnet does not influence kitty job dispatch | PASS | Kitty dispatch policy uses kitty-local reputation only |
| Bridge connection severed; both graphs remain operational independently | PASS | Local graph writes continue and private graph changes do not mutate mainnet reputation |

## Property Coverage

| Test | Property tested | Property NOT tested |
|---|---|---|
| Identity claim with master signature works on both graphs | One master public key can verify handle claims, private alias mappings, sovereign graph registration, and mid-slug reference parsing across the two synthetic graphs. | Real cryptographic key custody, email attestation, or live graph-registration persistence. |
| Reputation snapshot at kitty onboarding captures mainnet score | Kitty imports a signed mainnet snapshot exactly once at onboarding and reads the imported score. | Ongoing reputation propagation, production snapshot storage, or multi-operator registry arbitration. |
| Reputation snapshot import is one-shot | Duplicate snapshot import returns `SnapshotAlreadyImported`, and tampering with statement-bound snapshot bytes fails signature verification. | Real signature algorithms, key rotation over old snapshots, or cross-process write races. |
| Mainnet reputation evolution does not propagate post-snapshot | Mainnet reputation can change after import while kitty's imported snapshot remains frozen and kitty-local reputation evolves independently. | Production replication lag, background sync jobs, or adversarial registry forks. |
| Kitty contribution with `release_to_mainnet=true` surfaces on mainnet | A releasable kitty contribution with a mid-slug ref is mirrored into mainnet with its source slug recorded. | Live bridge transport, moderation policy, or duplicate bridge replay protection. |
| Credit transferred kitty to mainnet incurs bridge tax | Sovereign-mode transfer applies the 2 percent bridge friction at par and credits only the net amount. | Live settlement rows, exchange-rate changes, refunds, or fee distribution. |
| Compute job dispatched mainnet-side fulfills via kitty-side provider | A mainnet job can route through the bridge to a kitty provider and record a kitty completion. | Live provider execution, provider fraud proofs, or settlement clearing in `wire_settlements`. |
| Provider reputation in mainnet does not influence kitty job dispatch | Kitty dispatch selects by kitty-local reputation even when mainnet reputation strongly favors a different provider. | Full market ranking, queue pressure, or multi-signal provider scoring. |
| Bridge connection severed; both graphs remain operational independently | When the synthetic bridge is disconnected, both graphs still accept local writes and kitty-side changes do not mutate mainnet reputation. | Network partitions in live infrastructure, replay on reconnect, or operator dispute resolution. |

Acceptance:

```sh
cargo test --workspace
scripts/layer4-two-graph-bridged-synthetic.sh
```

Both commands should remain green before advancing to Layer 5 live-LLM-inference compute-market
roundtrip.
