# D3 Live Compute Settlement

D3 is the first full live substrate-to-mainnet compute settlement gate. It keeps the Layer 5 live LLM provider, but moves the roundtrip through the real GoodNewsEveryone compute-market HTTP surface, a Cloudflare tunnel, and the `wire_settlements` service-role read surface.

## Scope

- Reuse the persisted `agent-wire-substrate-node` mainnet Wire credential.
- Register separate provider and requester node rows for the Kramer agent.
- Provision and run a Cloudflare tunnel to a local substrate provider endpoint.
- Publish a real compute offer and queue mirror for `inception/mercury-2`.
- Quote, purchase, fill, and dispatch a small real-money compute job.
- Execute one live OpenRouter inference and post Wire settlement metadata.
- Verify the job reaches completed/settled and `wire_settlements` exposes the row.

The script accepts `D3_ENV_FILE=/path/to/.env` and sources it before invoking the CLI. Required secrets are read from environment only:

- `OPENROUTER_API_KEY`
- `SUPABASE_SERVICE_ROLE_KEY`
- `NEXT_PUBLIC_SUPABASE_URL` or `SUPABASE_URL`
- `D3_CLOUDFLARED_PATH` when `cloudflared` is not on `PATH`
- `D3_TUNNEL_HEALTH_TIMEOUT_SECS` to override the 120-second DNS/tunnel health window

D3 treats `/compute/fill` dispatch as a live tunnel propagation seam. Transient
`provider_unreachable`, `http_530`, and 502/503/504 fill failures are retried
with one stable idempotency key so the requester cannot be double-charged while
the Cloudflare route catches up. Retry policy controls:

- `D3_FILL_RETRY_TIMEOUT_SECS`: total retry window. Defaults to `180`.
- `D3_FILL_RETRY_MAX_ATTEMPTS`: hard attempt cap. Defaults to `24`.
- `D3_FILL_RETRY_BACKOFF_MILLIS`: base backoff. Defaults to `5000`.
- `D3_FILL_RETRY_MAX_JITTER_MILLIS`: deterministic per-attempt jitter cap. Defaults to `1000`.

## Command

```bash
D3_ENV_FILE="/path/to/.env" scripts/d3-live-compute-settlement.sh
```

## Passing Criteria

All D3 sub-tests must pass:

1. `d3-config-resolves`
2. `mainnet-auth-loads`
3. `local-provider-server-starts`
4. `provider-node-registers`
5. `requester-node-registers`
6. `cloudflare-tunnel-reachable`
7. `provider-offer-publishes-and-mirrors`
8. `requester-purchases-real-compute-job`
9. `requester-fill-dispatches-to-provider`
10. `provider-executes-and-posts-settlement`
11. `job-status-settled`
12. `wire-settlements-row-visible`

The CLI fails closed. If `/node/register` rejects because the live route is behind the current `wire_nodes` schema, the validator uses a service-role bootstrap fallback for the two temporary D3 node rows and reports the route residual in closeout.
