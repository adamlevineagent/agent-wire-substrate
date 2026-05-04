# L6 Stability Driver

The L6 driver is the substrate-side harness for long-running reference-client stability. It reuses the D3 live compute settlement validator as the canonical per-cycle primitive: every cycle provisions or reuses the mainnet surfaces required by D3, runs a Cloudflare-backed provider, buys/fills a small compute job, posts settlement, and verifies `wire_settlements`.

## Scope

- Run repeated small real-money D3 settlement cycles.
- Exercise Cloudflare tunnel lifecycle by creating a fresh provider tunnel per cycle.
- Record per-cycle job and settlement identifiers.
- Track cycle latency summaries and process high-water RSS snapshots via `getrusage`.
- Fail closed on the first unsettled, uncompleted, or otherwise red D3 cycle.

Newman owns the failure-injection and observability lane. This driver exposes the stable substrate loop and the per-cycle timing/settlement record Newman can coordinate against.

## Environment

The script accepts `L6_ENV_FILE=/path/to/.env` and sources it before invoking the CLI. It passes through the D3-required variables:

- `OPENROUTER_API_KEY`
- `SUPABASE_SERVICE_ROLE_KEY`
- `NEXT_PUBLIC_SUPABASE_URL` or `SUPABASE_URL`
- `D3_CLOUDFLARED_PATH` when `cloudflared` is not on `PATH`

L6-specific controls:

- `L6_CYCLES`: number of D3 cycles to run. Defaults to `1`.
- `L6_CYCLE_DELAY_SECS`: delay between cycles. Defaults to `0`.

## Command

```bash
L6_ENV_FILE="/path/to/.env" L6_CYCLES=12 L6_CYCLE_DELAY_SECS=30 scripts/l6-stability-driver.sh
```

## Passing Criteria

For the driver harness itself:

1. Every requested cycle passes D3.
2. Every completed cycle has a settled `wire_settlements` row.
3. The report includes p50/p99 cycle latency and RSS snapshots.
4. The process exits non-zero on the first failed cycle.

The full L6 mission remains the long-running 48-72 hour evidence window plus Newman-coordinated failure injection for provider/requester/tunnel kills.
