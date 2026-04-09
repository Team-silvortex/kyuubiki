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
- `KYUUBIKI_CLUSTER_API_TOKEN`
- `KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS`
- `KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS`
- `KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS`
- `KYUUBIKI_PROTECT_READS=true|false`

Behavior:

- no token configured
  local-friendly mode, reads and writes are open
- token configured
  mutating and cluster routes require the token
- cluster token configured
  cluster registration, heartbeat, and removal routes require the dedicated cluster token
- cluster agent allowlist configured
  only registered agent IDs in the allowlist may join or heartbeat
- cluster ID allowlist configured
  cluster routes require a matching allowed `cluster_id`
- cluster token omitted
  cluster routes fall back to `KYUUBIKI_API_TOKEN`
- cluster timestamp header present
  cluster routes reject stale requests outside the configured timestamp window
- `KYUUBIKI_PROTECT_READS=true`
  read routes should also be considered protected in deployment policy

### Direct mesh GUI

Direct mesh routes can now be disabled or token-protected:

- `KYUUBIKI_DIRECT_MESH_ENABLED=false`
- `KYUUBIKI_DIRECT_MESH_TOKEN=<token>`

This is important because direct mesh routes bypass Phoenix job persistence and
talk straight to solver agents.

### Frontend operator settings

The workbench can now carry optional operator tokens from browser-local
settings:

- control-plane token
- cluster token
- direct-mesh token

They are currently stored in browser local storage and attached as:

- `x-kyuubiki-token` to `/api/v1` and `/api/health`
- `x-kyuubiki-token` to `/api/direct-mesh/*`

The installer GUI can also write these environment variables into `.env.local`
for deployment setup.

## Operational recommendations

### Local workstation

- prefer `sqlite`
- keep direct mesh enabled only when needed
- do not expose the frontend or orchestrator to untrusted networks

### Central control plane

- use `postgres`
- set `KYUUBIKI_API_TOKEN`
- set `KYUUBIKI_CLUSTER_API_TOKEN` for remote node registration instead of reusing the main write token
- use `KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS` and `KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS` when you want a low-overhead membership gate without introducing certificates yet
- keep the cluster timestamp window short unless your deployment has unusual clock skew
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
- no signed cluster membership yet
- no per-project permissions
- no audit log retention policy yet

For now, think of the current token support as a deployment guardrail, not a
complete security architecture.
