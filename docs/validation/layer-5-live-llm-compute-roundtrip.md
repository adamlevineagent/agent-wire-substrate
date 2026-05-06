# Layer 5 Live LLM Compute Roundtrip

Layer 5 is the first non-synthetic substrate validation gate. It keeps the Layer 3 compute-market path, but swaps the deterministic echo adapter for a live LLM provider adapter. The validation is intentionally local and operator-driven: it performs one provider call, records the in-memory substrate receipts, and does not deploy, touch a live database, run live smoke, or publish npm packages.

## Scope

- Provider role registers a compute offer for a real model.
- Requester submits a `ComputeJobEnvelope` through the compute-market contract surface.
- Provider claims the job and executes it through an `ExecutionAdapter`.
- `ChronicleSink` records the delivery receipt.
- Requester reads the completion and settles credits within the job budget.

The default provider is a local LM Studio OpenAI-compatible server at
`http://127.0.0.1:1234/v1` with `gemma-4-e2b-it-mlx`. Override with:

- `LAYER5_PROVIDER=lm_studio` or `LAYER5_PROVIDER=openrouter`
- `LM_STUDIO_BASE_URL`
- `LM_STUDIO_MODEL` or `LAYER5_MODEL`
- `OPENROUTER_API_KEY` when `LAYER5_PROVIDER=openrouter`
- `OPENROUTER_MODEL` and `OPENROUTER_BASE_URL` for OpenRouter overrides

The script also accepts `LAYER5_ENV_FILE=/path/to/.env` and sources it before invoking the node CLI. Secrets are never committed or hardcoded.

## Command

```bash
LAYER5_ENV_FILE="/path/to/.env" scripts/layer5-live-llm-compute-roundtrip.sh
```

or, if the environment is already populated:

```bash
scripts/layer5-live-llm-compute-roundtrip.sh
```

## Passing Criteria

All five sub-tests must pass:

1. `provider-registers-live-model-offer`
2. `requester-submits-compute-job-envelope`
3. `provider-executes-inference-via-execution-adapter`
4. `chronicle-records-live-completion`
5. `requester-reads-completion-and-settles`

The live adapter prompts the model to return `SUBSTRATE_ROUNDTRIP_OK`. The content sanity check accepts the response only if that sentinel is present.

The default cap is 160 output tokens. Lower caps can produce provider responses
without usable text on reasoning-oriented backends even when the request itself
succeeds.

## Non-Live Test Behavior

Workspace tests use a deterministic in-memory adapter. That preserves CI safety and proves the substrate roundtrip contract without spending live LLM credits. The CLI command is the live gate and exits non-zero if provider config is missing or the model call/content/settlement fails.
