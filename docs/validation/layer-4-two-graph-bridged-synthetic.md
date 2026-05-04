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
| Mainnet reputation evolution does not propagate post-snapshot | PASS | The reputation firewall is one-shot and asymmetric |
| Kitty contribution with `release_to_mainnet=true` surfaces on mainnet | PASS | Bridge release policy carries a mid-slug contribution into mainnet |
| Credit transferred kitty to mainnet incurs bridge tax | PASS | Sovereign-mode 2 percent bridge friction applies at par |
| Compute job dispatched mainnet-side fulfills via kitty-side provider | PASS | Cross-graph compute-market routing can reach a kitty provider |
| Provider reputation in mainnet does not influence kitty job dispatch | PASS | Kitty dispatch policy uses kitty-local reputation only |
| Bridge connection severed; both graphs remain operational independently | PASS | Local graph writes continue and private graph changes do not mutate mainnet reputation |

Acceptance:

```sh
cargo test --workspace
scripts/layer4-two-graph-bridged-synthetic.sh
```

Both commands should remain green before advancing to Layer 5 live-LLM-inference compute-market
roundtrip.
