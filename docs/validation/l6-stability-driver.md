# L6 Stability Driver

The L6 driver is the substrate-side harness for long-running reference-client stability. It reuses the D3 live compute settlement validator as the canonical per-cycle primitive: every cycle provisions or reuses the mainnet surfaces required by D3, runs a Cloudflare-backed provider, buys/fills a small compute job, posts settlement, and verifies `wire_settlements`.

Because each L6 cycle creates a fresh Cloudflare route, D3's fill step uses a
bounded retry policy for transient tunnel-propagation dispatch failures. Retries
reuse the same fill idempotency key and are capped by timeout, max attempts,
base backoff, and deterministic jitter.

## Scope

- Run repeated small real-money D3 settlement cycles.
- Exercise Cloudflare tunnel lifecycle by creating a fresh provider tunnel per cycle.
- Record per-cycle job and settlement identifiers.
- Track cycle latency summaries and process high-water RSS snapshots via `getrusage`.
- Run read-only observability scans over green cycles for leaked claims,
  orphan settlements, duplicate settlements, settlement-shape drift, throughput,
  and RSS deltas.
- Fail closed on the first unsettled, uncompleted, or otherwise red D3 cycle.

Newman's observability and failure-injection lane is integrated as an additive
seam. The stability driver now prints the observability report after each run,
and `l6-failure-injection` exercises the substrate recovery policy across the
five kill-points.

## Environment

The script accepts `L6_ENV_FILE=/path/to/.env` and sources it before invoking the CLI. It passes through the D3-required variables:

- `SUPABASE_SERVICE_ROLE_KEY`
- `NEXT_PUBLIC_SUPABASE_URL` or `SUPABASE_URL`
- D3 live LLM provider controls documented in `docs/validation/d3-live-compute-settlement.md`
- D3 fill retry controls documented in `docs/validation/d3-live-compute-settlement.md`

By default, L6 inherits D3's local LM Studio provider
(`http://127.0.0.1:1234/v1`, `granite-4-micro`) and the transport crate's
bundled/downloaded cloudflared resolution. It does not require
`OPENROUTER_API_KEY` or a cloudflared path environment override.

L6-specific controls:

- `L6_CYCLES`: number of D3 cycles to run. Defaults to `1`.
- `L6_CYCLE_DELAY_SECS`: delay between cycles. Defaults to `0`.

## Command

```bash
L6_ENV_FILE="/path/to/.env" L6_CYCLES=12 L6_CYCLE_DELAY_SECS=30 scripts/l6-stability-driver.sh
```

Run deterministic recovery-policy kill-point scenarios:

```bash
scripts/l6-failure-injection.sh
```

## Passing Criteria

For the driver harness itself:

1. Every requested cycle passes D3.
2. Every completed cycle has a settled `wire_settlements` row.
3. The report includes p50/p99 cycle latency and RSS snapshots.
4. The observability report has `all_invariants_held=true`.
5. The process exits non-zero on the first failed cycle or invariant violation.

For recovery-policy injection:

1. Before-provider-claim recovery produces exactly one preserved claim.
2. After-provider-claim recovery rejects duplicate claims.
3. After-claim-before-completion recovery allows one completion and rejects
   duplicate completions.
4. After-settlement recovery rejects duplicate settlement.
5. During-tunnel-rotation recovery rejects a duplicate claim through the
   rotated tunnel.

The full L6 mission remains the long-running 48-72 hour evidence window plus Newman-coordinated failure injection for provider/requester/tunnel kills.
