# V1 WS0 Tunnel Supervisor Slice

This slice starts WS0 by moving the Cloudflare tunnel lifecycle from a static URL
adapter toward the reusable substrate supervisor shape that V1 needs before live
compute-market activity.

Shipped in `agent-wire-transport-cloudflare`:

- Persistent `TunnelState` with tolerant `tunnel_url` deserialization, preserving
  `tunnel_id` and `tunnel_token` when a saved URL is malformed.
- Cloudflared binary path selection, build-time bundle discovery,
  node-resource/data-dir/PATH fallback, platform-specific download URL
  selection, and download/extract helper.
- Server-side tunnel provisioning via `POST /api/v1/node/tunnel`.
- Wire-native `tunnel_state/<node-id>.md` load/save helpers with legacy JSON
  read fallback.
- Stale credential detection for the stable
  `https://node-{nodeId}.agent-wire.com` URL pattern.
- Resolver that reuses valid persisted tokens or provisions and persists a new
  tunnel state.
- Cloudflared child-process spawn through the resolver, pre-spawn orphan cleanup, and stderr
  classifier for connected/error markers.
- `CloudflareTunnelDriver::from_state` so existing `TransportDriver` consumers
  can open sessions from supervised state.

Validation:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo test -p agent-wire-transport-cloudflare
cargo test --lib --workspace
```

No live tunnel provisioning, cloudflared download, process spawn, deploy, npm
publish, crates.io publish, or L6 cron/long-runner start is part of this slice.
