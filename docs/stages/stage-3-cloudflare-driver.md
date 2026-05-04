# Stage 3: Cloudflare Driver Against Foundation Transport

Status: ready for ship-through after Stage 1 integration lands on `origin/main`

## Scope

Stage 3 binds `crates/transport-cloudflare` to the foundation transport trait.
The driver crate depends on `agent-wire-foundation`; foundation does not import
the driver crate.

The implemented surface is intentionally scaffolding-tier:

- `EndpointUrl`, `CallbackUrl`, and `TunnelUrl` validate HTTP/HTTPS URLs through
  a structured URL parser.
- `TunnelRequest` carries the local endpoint, optional requested public URL, and
  callback URLs.
- `TunnelSession` returns the chosen public URL, local endpoint, callback list,
  and driver name.
- `TransportDriver::open_tunnel` is the canonical trait entrypoint for driver
  crates.
- `CloudflareTunnelDriver` implements `TransportDriver` using either a static
  configured tunnel URL or a requested public URL.

## Non-Goals

- No live tunnel process orchestration.
- No cloudflared binary management.
- No deploy, live smoke, npm publish, or production mutation.
- No compute-market, storage-market, relay-market, or node composition work.

## Guardrails

Foundation dependency-direction guards run under `cargo test --lib` and as the
existing integration test. The guard forbids foundation references to downstream
market, node, driver, Tauri, Cloudflare, or pyramid vocabulary.

## Verification

Before ship-through merge, run:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test --lib --workspace
cargo test --workspace
git diff --check
```
