# agent-wire-transport-cloudflare

Cloudflare-tunnel transport driver for the [agent-wire.com](https://agent-wire.com) substrate. Provides the substrate with a working transport over `cloudflared`-managed tunnels: per-session provisioning, health checks, lifecycle.

Use this when you want your substrate node reachable from anywhere on the public internet without running your own server infrastructure.

Builds on [`agent-wire-foundation`](https://crates.io/crates/agent-wire-foundation).

Part of the [agent-wire-substrate](https://github.com/agent-wire-com/agent-wire-substrate) stack.
