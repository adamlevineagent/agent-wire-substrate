# D3 Live Compute Settlement

D3 is the first full live substrate-to-mainnet compute settlement gate. It keeps the Layer 5 live LLM provider, but moves the roundtrip through the real GoodNewsEveryone compute-market HTTP surface, a Cloudflare tunnel, and the canonical Wire settlement API.

## Scope

- Reuse the persisted `agent-wire-substrate-node` mainnet Wire credential.
- Register separate provider and requester node rows for the Kramer agent.
- Provision and run a Cloudflare tunnel to a local substrate provider endpoint.
- Publish a real compute offer and queue mirror for the configured live model.
- Quote, purchase, fill, and dispatch a small real-money compute job.
- Execute one live OpenAI-compatible inference and post Wire settlement metadata.
- Verify the job reaches completed/settled and `GET /api/v1/wire/settlements?job_id=<id>` exposes the settlement through the same persisted Wire credential.

The script accepts `D3_ENV_FILE=/path/to/.env` and sources it before invoking
the CLI. D3 defaults to local LM Studio at `http://127.0.0.1:1234/v1` with
`granite-4-micro`, so the live LLM path no longer requires an OpenRouter key.
D3 reuses the persisted substrate-node Wire credential. The mainnet endpoint
defaults to `https://newsbleach.com/api/v1`, and may be overridden with
`WIRE_MAINNET_ENDPOINT`. D3 has no settlement database secret. Optional runtime
controls:

- `D3_TUNNEL_HEALTH_TIMEOUT_SECS` to override the 120-second DNS/tunnel health window

Provider controls:

- `D3_LLM_PROVIDER=lm_studio` or `D3_LLM_PROVIDER=openrouter`
- `LM_STUDIO_BASE_URL`
- `D3_MODEL`, `LM_STUDIO_MODEL`, or `LAYER5_MODEL`
- `OPENROUTER_API_KEY` only when `D3_LLM_PROVIDER=openrouter`
- `OPENROUTER_MODEL` and `OPENROUTER_BASE_URL` for OpenRouter overrides

Cloudflared is resolved by the transport crate from a bundled build artifact,
the node resource directory, the node data directory, `PATH`, or a fresh
platform download. D3 does not accept or require a cloudflared path environment
override.

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
12. `canonical-settlement-api-visible`

The CLI fails closed. Settlement verification uses the canonical Wire HTTP route
with the persisted agent bearer token; D3 does not accept database-admin
credentials or a direct database settlement read path.
