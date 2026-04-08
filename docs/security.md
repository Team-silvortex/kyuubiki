# Security Notes

Kyuubiki is still in an engine-building phase, so the default local developer
experience is intentionally low friction. For anything beyond a trusted local
machine or small trusted LAN, enable the guardrails below.

## Current security model

### Control plane

The orchestrator can now enforce an API token for:

- mutating HTTP routes
- cluster registration routes

Supported request headers:

- `Authorization: Bearer <token>`
- `x-kyuubiki-token: <token>`

Environment variables:

- `KYUUBIKI_API_TOKEN`
- `KYUUBIKI_PROTECT_READS=true|false`

Behavior:

- no token configured
  local-friendly mode, reads and writes are open
- token configured
  mutating and cluster routes require the token
- `KYUUBIKI_PROTECT_READS=true`
  read routes should also be considered protected in deployment policy

### Direct mesh GUI

Direct mesh routes can now be disabled or token-protected:

- `KYUUBIKI_DIRECT_MESH_ENABLED=false`
- `KYUUBIKI_DIRECT_MESH_TOKEN=<token>`

This is important because direct mesh routes bypass Phoenix job persistence and
talk straight to solver agents.

## Operational recommendations

### Local workstation

- prefer `sqlite`
- keep direct mesh enabled only when needed
- do not expose the frontend or orchestrator to untrusted networks

### Central control plane

- use `postgres`
- set `KYUUBIKI_API_TOKEN`
- disable direct mesh unless explicitly required
- place the orchestrator behind TLS termination or a trusted reverse proxy

### Distributed or LAN mesh

- treat Rust solver agents as privileged compute nodes
- use trusted subnets or VPN only
- do not expose solver RPC directly to the public internet
- prefer control-plane mediation unless direct mesh is explicitly needed

## Current gaps

These areas are not yet finished security features:

- no built-in TLS for solver RPC
- no multi-user authn/authz model
- no signed cluster membership
- no per-project permissions
- no audit log retention policy yet

For now, think of the current token support as a deployment guardrail, not a
complete security architecture.
