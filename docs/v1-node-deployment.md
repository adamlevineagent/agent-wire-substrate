# agent-wire-substrate-node V1 Deployment

`agent-wire-substrate-node` is the V1 reference participant for the Agent Wire
substrate. V1 deployment means installing and validating the CLI binary plus its
local state, auth, protocol surfaces, and live-validation gates. It does not
mean deploying GoodNewsEveryone, applying database migrations, publishing npm
packages, or arming the L6 calendar.

## Build

From a clean workspace:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo build --release
test -x target/release/agent-wire-substrate-node
```

Install the binary wherever the operator keeps local tools:

```sh
install -m 0755 target/release/agent-wire-substrate-node ~/.local/bin/agent-wire-substrate-node
agent-wire-substrate-node --help
```

During development, use Cargo instead:

```sh
cargo run -p agent-wire-substrate-node -- surface
```

## Local State

The node keeps local state under `~/.wire-node/state` by default. V1 local
state uses the same wire-native document convention as canonical Wire docs:
frontmatter, a typed YAML payload document, then optional operator prose. The
foundation codec owns record-kind/schema validation, typed refs, timestamping,
atomic writes, and private-file permissions for secret-bearing records.

| File | Owner | Purpose |
| --- | --- | --- |
| `mainnet_auth_credential/agent-wire-substrate-node.md` | `auth` and live validation commands | Validated mainnet credential state; private `0600` on Unix. |
| `identity_snapshot/v1-identity.md` | `identity persist/load` and `runtime smoke` | Compact V1 node identity state. |
| `tunnel_state/<node-id>.md` | Cloudflare transport resolver | Persisted tunnel id, URL, token, and lifecycle status; private `0600` on Unix. |

`AGENT_WIRE_NODE_STATE_DIR` overrides the V1 runtime state directory when no
explicit state path is provided. `WIRE_AUTH_STATE_PATH` overrides the mainnet
auth-state path. Token material must stay in the environment, token files, or
private local-state docs; the CLI should not print it. The auth loader can read
the legacy `agent-wire-substrate-node-auth.json` file and rewrites it as a
wire-native local-state document after validation.

## Bootstrap Auth

Use one of the supported bootstrap sources:

```sh
WIRE_API_TOKEN=... agent-wire-substrate-node auth
WIRE_API_TOKEN_FILE=/path/to/token agent-wire-substrate-node auth
WIRE_DEVICE_SECRET=... agent-wire-substrate-node auth
WIRE_OPERATOR_EMAIL=hello@callmeplayful.com agent-wire-substrate-node auth
```

Optional auth configuration:

- `WIRE_MAINNET_ENDPOINT`: Wire API base URL.
- `WIRE_AGENT_NAME`: requested agent name. Defaults to
  `agent-wire-substrate-node`.
- `WIRE_AUTH_STATE_PATH`: auth-state file override.

A second `agent-wire-substrate-node auth` run without bootstrap secrets should
reuse the persisted state.

## Local V1 Smoke

Run the V1 surface smoke before any live validation:

```sh
scripts/v1-node-surface.sh
agent-wire-substrate-node runtime smoke ~/.wire-node/state
```

These commands prove the CLI manifests, canonical chain loader, MCP and HTTP
dispatch adapters, loopback HTTP listener, identity store, and maintenance
scheduler. They intentionally do not touch live settlement or release surfaces.

## Live Validation

Live validation is environment-gated and should use small real-money amounts.

```sh
agent-wire-substrate-node contribution-sync
agent-wire-substrate-node layer5-live-llm
agent-wire-substrate-node d3-live-compute-settlement
agent-wire-substrate-node l6-stability-driver
```

Current live validation secrets are read from environment only until addendum
B/C/D lands:

- `OPENROUTER_API_KEY` for live model calls.
- `OPENROUTER_BASE_URL`, `OPENROUTER_MODEL`, or `LAYER5_MODEL` to override
  provider defaults.
- `SUPABASE_SERVICE_ROLE_KEY` for D3/L6 settlement verification.
- `NEXT_PUBLIC_SUPABASE_URL` or `SUPABASE_URL` for the settlement backend.
- `D3_CLOUDFLARED_PATH` when `cloudflared` is not on `PATH`.

Optional D3 controls:

- `D3_MODEL`
- `D3_MAX_TOKENS`
- `D3_MAX_BUDGET`
- `D3_TUNNEL_HEALTH_TIMEOUT_SECS`
- `D3_FILL_RETRY_TIMEOUT_SECS`
- `D3_FILL_RETRY_MAX_ATTEMPTS`
- `D3_FILL_RETRY_BACKOFF_MILLIS`
- `D3_FILL_RETRY_MAX_JITTER_MILLIS`

The L6 calendar remains a separate acceptance window. Do not install or arm L6
cron until the tunnel-health blocker is resolved or Partner/Jerry explicitly
instructs it.

## Cloudflared Gate

`agent-wire-transport-cloudflare` contains the platform-aware cloudflared
download and lifecycle primitives. D3 currently accepts `D3_CLOUDFLARED_PATH`
or a `cloudflared` found on `PATH`.

The V1 release gate is stricter than local D3 convenience: a clean-machine
release proof must show that the node can acquire or bundle cloudflared without
depending on a developer-local preinstall. Do not mark V1 ready for publish
until that evidence exists.

## Release Gate

Before requesting v1.0.0 publish:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --lib --workspace
cargo test --workspace
cargo build --release
scripts/substrate-node-demo.sh
scripts/v1-node-surface.sh
git diff --check
```

Then run the live integration evidence requested by Partner/Jerry:

- mainnet auth bootstrap and restart reuse;
- canonical chain compile/quote/execute;
- live compute settlement through D3;
- settlement read-back from `wire_settlements`;
- final Newman/Elaine audit;
- clean-machine cloudflared proof.

All actual `cargo publish`, npm publish, deploy, and database operations remain
Adam/`@playful` gated.
