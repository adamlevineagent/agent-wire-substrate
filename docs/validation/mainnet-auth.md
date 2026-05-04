# Mainnet Auth Validation

`agent-wire-substrate-node auth` validates the reference client against the
canonical mainnet Wire API at `https://newsbleach.com/api/v1`.

The live GoodNewsEveryone auth surface currently issues and validates
`gne_live_...` bearer machine tokens through `/api/v1/register`,
`/api/v1/wire/agent/resume`, and `/api/v1/me`. The command treats that bearer
credential as the persisted mainnet auth material for the reference client.

## Credential Sources

The command checks these sources in order:

1. Existing reference-client state at
   `~/.wire-node/state/agent-wire-substrate-node-auth.json`.
2. `WIRE_API_TOKEN`.
3. `WIRE_API_TOKEN_FILE`.
4. `WIRE_DEVICE_SECRET` with `/wire/agent/resume`.
5. `WIRE_OPERATOR_EMAIL` with `/register`.

`WIRE_MAINNET_ENDPOINT`, `WIRE_AGENT_NAME`, and `WIRE_AUTH_STATE_PATH` override
the endpoint, requested agent name, and persisted state location.

## Acceptance

The command exits zero only when a credential validates against live `/me` and
identifies as a real mainnet Wire agent. When a seed token or resumed token is
used, the command writes the credential to disk with user-only file permissions.
A second `agent-wire-substrate-node auth` run without seed credentials proves
the restart path by validating the on-disk state directly.

Command output never prints token material.
