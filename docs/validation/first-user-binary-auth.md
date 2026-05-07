# First-User Binary Auth Slice

## Scope

This slice closes the first-user binary auth gaps called out as F-004, F-005,
and F-007:

- `identity login` is a real auth path, not a placeholder surface binding.
- `identity resume` is an explicit alias for the persisted/device-secret resume
  path.
- Auth token persistence can use either private-file local state or an OS
  credential-store reference.

## Behavior

`agent-wire-substrate-node identity login` and
`agent-wire-substrate-node identity resume` both run the same mainnet auth state
machine as `agent-wire-substrate-node auth`:

1. validate an existing persisted credential;
2. validate `WIRE_API_TOKEN` or `WIRE_API_TOKEN_FILE`;
3. resume with `WIRE_DEVICE_SECRET`;
4. register with `WIRE_OPERATOR_EMAIL`.

The commands exit non-zero if none of those credential paths can produce a
validated mainnet identity. Output redacts token material.

## Secret Backend

`WIRE_AUTH_SECRET_BACKEND` accepts `auto`, `private-file`, or `keychain`.

- `auto` keeps the existing Unix behavior: a private `0600` wire-native local
  state doc.
- On Windows, `auto` uses OS credential storage so first-run auth does not depend
  on Unix permission bits.
- `keychain` stores the token in the platform credential store and persists only
  a `KeychainRef` in the local-state doc.

## Validation

Run these before shipping the slice:

```sh
cargo test -p agent-wire-substrate-node mainnet_auth
cargo test -p agent-wire-substrate-node v1_surface
cargo check --workspace
scripts/v1-node-surface.sh
```

Live validation remains gated by credentials and should use the existing
`agent-wire-substrate-node auth` or `identity login` path only when the operator
has provided a bootstrap credential.
