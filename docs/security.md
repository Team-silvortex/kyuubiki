# Security Notes

Kyuubiki is still in an engine-building phase, so the default local developer
experience is intentionally low friction. For anything beyond a trusted local
machine or small trusted LAN, enable the guardrails below.

Security-sensitive paths are explicitly marked in:

- `security-sensitive-modules.md`

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
- `KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT=true|false`
- `KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS`
- `KYUUBIKI_PROTECT_READS=true|false`

Behavior:

- no token configured
  local-friendly mode, reads and writes are open
- token configured
  mutating and cluster routes require the token
- token configured on non-local deployments
  read routes are also protected by default unless `KYUUBIKI_PROTECT_READS=false`
- cluster token configured
  cluster registration, heartbeat, and removal routes require the dedicated cluster token
- cluster agent allowlist configured
  only registered agent IDs in the allowlist may join or heartbeat
- cluster ID allowlist configured
  cluster routes require a matching allowed `cluster_id`
- cluster fingerprint required
  cluster routes require `x-kyuubiki-agent-fingerprint`, and an already-registered agent ID may only heartbeat or unregister with the same fingerprint
- cluster token omitted
  cluster routes fall back to `KYUUBIKI_API_TOKEN`
- cluster timestamp header present
  cluster routes reject stale requests outside the configured timestamp window
- `KYUUBIKI_PROTECT_READS=true`
  read routes should also be considered protected in deployment policy

Protected read routes now include:

- `/api/health`
- `/api/v1/protocol*`
- `/api/v1/agents`
- `/api/v1/jobs*`
- `/api/v1/results*`
- `/api/v1/export/database`
- `/api/v1/projects*`
- `/api/v1/models*`
- `/api/v1/model-versions*`

### Direct mesh GUI

Direct mesh routes can now be disabled or token-protected:

- `KYUUBIKI_DIRECT_MESH_ENABLED=false`
- `KYUUBIKI_DIRECT_MESH_TOKEN=<token>`
- `KYUUBIKI_DIRECT_MESH_ENDPOINTS=host:port,...`
- `KYUUBIKI_DIRECT_MESH_ALLOW_REQUEST_ENDPOINTS=true|false`

This is important because direct mesh routes bypass Phoenix job persistence and
talk straight to solver agents.

Endpoint policy:

- `local` deployment
  request-defined direct-mesh endpoints are allowed by default
- `cloud` or `distributed` deployment
  request-defined endpoints are denied by default; direct mesh requests must use
  the environment-configured endpoint list unless
  `KYUUBIKI_DIRECT_MESH_ALLOW_REQUEST_ENDPOINTS=true`

### Frontend operator settings

The workbench can now carry optional operator tokens from browser-local
settings:

- control-plane token
- cluster token
- direct-mesh token

They are now stored in browser session storage, while non-sensitive UI
preferences remain in local storage. Legacy local-storage tokens are migrated
out on load.

Assistant-planned actions and WASM Python scripted actions now share the same
high-risk confirmation gate inside the workbench action executor. Destructive
or sensitive actions such as project/model deletion, job cancellation, and
database or project export require an explicit operator confirmation even when
triggered through the assistant or the scripting bridge.

The workbench also keeps a session-scoped security audit trail for these
high-risk automation actions so operators can see whether they were prompted,
cancelled, completed, or failed during the current browser session.

When the control plane is reachable, these high-risk action events are also
posted into the orchestrator's append-only security event stream and included
in database exports under `security_events`.

For analysis-oriented workflows, Kyuubiki now also exposes a dedicated
`GET /api/v1/export/security-events` endpoint that returns:

- export timestamp
- normalized export schema metadata
- applied filter echo
- compact source/risk/status summary counts
- the filtered event list itself

There is also a companion `GET /api/v1/export/security-events.csv` endpoint for
flat spreadsheet and notebook workflows. It exports a stable row shape with
top-level event fields plus `study_kind`, `project_id`, and `model_version_id`
lifted out of event context.

The persisted security-event stream now supports filtered reads by:

- `source`
- `risk`
- `status`
- `action`
- `study_kind`
- `project_id`
- `model_version_id`
- `occurred_after`
- `occurred_before`

The runtime workbench panel consumes these filters to provide:

- time-window views such as last hour, day, week, or month
- compact event aggregates by risk and status
- lightweight study, project, and model-version facets for operator review
- simple trend buckets and source-by-status summaries for quick operator triage

Attached as:

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
- leave read protection enabled for `cloud` and `distributed` deployments unless
  you have a deliberate reverse-proxy or network-isolation reason not to
- set `KYUUBIKI_CLUSTER_API_TOKEN` for remote node registration instead of reusing the main write token
- use `KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS` and `KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS` when you want a low-overhead membership gate without introducing certificates yet
- turn on `KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT=true` when you want a low-friction binding between an agent ID and a specific node identity without going all the way to PKI
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
