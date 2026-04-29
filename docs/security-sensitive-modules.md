# Security-Sensitive Modules

This file marks the parts of Kyuubiki that should be reviewed with a security
mindset before changing behavior, dependencies, request shapes, or deployment
defaults.

Sensitivity levels:

- `critical`
  A bug can expose credentials, run remote commands, bypass authorization, or
  allow untrusted compute/network access.
- `high`
  A bug can expose persisted user data, corrupt project history, expand network
  reachability, or make unsafe automation easier.
- `moderate`
  A bug is less likely to be directly exploitable, but can weaken auditability,
  resource control, or operational safety.

## Critical Modules

| Path | Why it is sensitive | Review focus |
| --- | --- | --- |
| `apps/web/lib/kyuubiki_web/security.ex` | Central token, cluster identity, fingerprint, allowlist, and timestamp authorization helpers. | Never weaken token comparison, identity header matching, allowlist behavior, or replay-window checks without an explicit migration note. |
| `apps/web/lib/kyuubiki_web/router.ex` | Public HTTP route boundary for jobs, results, projects, agent registry, and database export. | Every mutating route must use `with_auth(conn, :write, ...)`; cluster routes must use `:cluster`; sensitive reads must be deliberately classified. |
| `apps/web/config/config.exs` | Runtime source of API tokens, cluster tokens, database URLs, storage backend, agent discovery, and security toggles. | Avoid logging secrets; treat default open local mode as development-only; keep deployment defaults explicit. |
| `apps/frontend/src/lib/direct-mesh/security.ts` | Token gate for direct mesh Next.js routes. | Keep direct mesh disable/token checks centralized and required before any direct solver connection. |
| `apps/frontend/src/app/api/direct-mesh/**/route.ts` | Browser-facing endpoints that can connect directly to solver agents and fetch cached direct-mesh results. | Validate request shape, require `authorizeDirectMeshRequest`, avoid exposing arbitrary internal networks in hosted deployments. |
| `apps/frontend/src/lib/direct-mesh/rpc.ts` | Node TCP client that opens sockets to solver agents selected by frontend/operator input. | Endpoint validation, timeouts, SSRF-style reachability, and unbounded response handling. |
| `apps/installer-gui/src-tauri/src/main.rs` | Tauri commands can write env files, run builds, and execute remote SSH bootstrap/agent commands. | Shell escaping, SSH target validation, command construction, secret handling, and capability scope. |
| `scripts/kyuubiki` | Operator entry point that starts services, exports DB snapshots, and uses shell `eval` for runtime env setup. | Shell quoting, env construction, token propagation, DB export destination, and local-only assumptions. |
| `workers/rust/crates/cli/src/main.rs` | Headless solver agent server and remote registration/heartbeat client. | RPC listener exposure, cluster token headers, fingerprint propagation, cancellation, and progress heartbeat behavior. |

## High-Sensitivity Modules

| Path | Why it is sensitive | Review focus |
| --- | --- | --- |
| `apps/web/lib/kyuubiki_web/playground/agent_registry.ex` | Stores and updates remote agent identity, endpoint, health, cluster metadata, and fingerprint. | Registration validation, stale-agent behavior, fingerprint persistence, and endpoint trust. |
| `apps/web/lib/kyuubiki_web/playground/agent_pool.ex` | Chooses solver agents from static config, manifests, and registry state. | Manifest parsing, failover behavior, remote endpoint trust, and deployment-mode separation. |
| `apps/web/lib/kyuubiki_web/playground/agent_client.ex` | Orchestrator TCP client to Rust solver agents. | Timeouts, frame parsing, error propagation, and network boundary assumptions. |
| `apps/web/lib/kyuubiki_web/storage/**` | SQLite/Postgres repos, schema setup, and persistence records for jobs, results, projects, and model versions. | Migration safety, data export scope, result payload size, and accidental sensitive-data logging. |
| `apps/web/lib/kyuubiki_web/jobs/**` and `apps/web/lib/kyuubiki_web/results/**` | Job/result persistence and watchdog-driven lifecycle state. | Cancellation semantics, stale job handling, result chunk boundaries, and operator edits. |
| `apps/frontend/src/lib/workbench/helpers.ts` | Workbench settings bridge persistent UI prefs and session-scoped secrets for operator tokens and LLM API keys. | Session storage behavior, migration of legacy secrets, token redaction, export boundaries, and accidental serialization of secrets. |
| `apps/frontend/src/components/workbench/system/workbench-system-config-card.tsx` | UI surface for entering operator tokens and exporting database snapshots. | Password field behavior, copy/export affordances, and avoiding accidental display of token values. |
| `apps/frontend/src/lib/api/index.ts` | Frontend API client attaches operator tokens to orchestrator and direct-mesh requests. | Header construction, session-secret lookup, token scope separation, and error handling without leaking secrets. |
| `apps/frontend/src/lib/assistant/openai-compatible.ts` | Optional LLM integration receives workbench context and returns executable action plans. | Prompt data minimization, API key storage, action validation, and tool-result redaction. |
| `apps/frontend/src/lib/scripting/workbench-script-runtime.ts` | Scriptable workbench actions include project/model CRUD and runtime operations. | Action allowlist, destructive actions, confirmation strategy, and future WASM Python sandboxing. |
| `apps/frontend/src/components/workbench/workbench.tsx` | Central workbench action executor is shared by manual UI actions, assistant plans, and WASM Python bridge calls. | Keep the high-risk confirmation gate and session audit logging centralized; do not add bypass paths for destructive/export actions. |
| `apps/frontend/src/lib/workbench/security-audit.ts` | Session-scoped audit trail for high-risk assistant and scripting actions. | Keep storage session-bounded, avoid secret leakage in notes, and preserve event ordering for prompt/cancel/complete/fail states. |
| `apps/web/lib/kyuubiki_web/security_events/**` | Append-only control-plane event stream for high-risk automation actions. | Keep validation strict, preserve append-only semantics, and avoid storing secrets in event context or notes. |
| `apps/frontend/src/components/workbench/workbench-assistant-panel.tsx` and `apps/frontend/src/components/workbench/workbench-script-panel.tsx` | UI surfaces that expose executable automation actions to operators. | Show risk state clearly, avoid “silent execute” affordances, and keep action metadata aligned with runtime guardrails. |
| `apps/frontend/src/lib/models/model-import.ts` | Imports external model JSON into solver/project state. | Schema validation, size limits, numeric bounds, and safe evolution across model schema versions. |
| `apps/frontend/src/lib/projects/**` | Project bundle import/export. | Archive/file parsing, path traversal prevention, payload size, and result/data export scope. |
| `apps/frontend/src/lib/materials/material-library.ts` | External material library import. | CSV/JSON parsing, numeric bounds, duplicate IDs, and maliciously large files. |
| `sdks/python/kyuubiki_sdk/auth.py`, `sdks/elixir/lib/kyuubiki_sdk/auth.ex`, `sdks/rust/src/auth.rs` | Token/header construction used by external automation and AI clients. | Header parity with control plane, token scoping, and clear examples that avoid hardcoding secrets. |
| `deploy/agents.*.json` and `schemas/agent-manifest.schema.json` | Static distributed-agent manifests. | Do not commit real hostnames/secrets; validate endpoint shape and intended deployment mode. |

## Moderate-Sensitivity Modules

| Path | Why it is sensitive | Review focus |
| --- | --- | --- |
| `docs/security.md` | Operator-facing security model and known gaps. | Keep it synchronized with actual route enforcement and deployment behavior. |
| `docs/packaging-and-deployment.md` | Deployment instructions can normalize unsafe defaults if stale. | Call out local-only defaults, token requirements, and direct mesh exposure clearly. |
| `workers/rust/crates/protocol/src/lib.rs` | Shared RPC frame model between orchestrator/frontend and Rust agents. | Backward compatibility, frame size assumptions, and error payload shape. |
| `tests/integration/*agent*` and `tests/integration/*direct-mesh*` | Tests encode expected auth and deployment behavior. | Keep smoke tests aligned with current guardrails, especially token-protected routes. |

## Current Review Notes

- Read routes in `apps/web/lib/kyuubiki_web/router.ex` are now wrapped with
  `with_auth(conn, :read, ...)`, and non-local deployments default to
  `protect_reads? = true` unless explicitly overridden. Keep new GET routes on
  the same path discipline.
- `GET /api/v1/export/database` returns a full database snapshot and is
  still a high-value read endpoint even after read-route protection. Treat
  changes there as sensitive and prefer additional network or proxy controls in
  real deployments.
- `GET /api/v1/export/security-events` is narrower than a full database export,
  but still exposes operator activity history and deployment context. Treat its
  schema, filtering, and auth behavior as security-sensitive.
- `GET /api/v1/export/security-events.csv` shares the same exposure boundary as
  the JSON security-event export. Keep the flattened column set stable and do
  not accidentally widen context leakage when adding new exported fields.
- Direct mesh is intentionally powerful: it bypasses Phoenix job persistence and
  opens TCP sockets from the Next.js server process to solver agents. Keep it
  disabled or token-protected outside trusted local/LAN environments. In
  non-local deployments, keep request-defined endpoints disabled unless you
  deliberately want operators to probe a broader agent surface.
- Remote deployment through the Tauri installer uses SSH and shell command
  construction. Treat all changes there as critical even when inputs appear to
  be operator-only.
- Browser-local token storage is convenient for workstation mode, not a
  multi-user auth model. Do not serialize those settings into project/model
  exports.
- Assistant and script actions now depend on a centralized confirmation gate
  for high-risk operations. Keep new destructive or export-capable actions
  classified in the shared action catalog before exposing them to automation.

## Change Checklist

Before changing any critical or high-sensitivity module:

1. Identify whether the change affects local-only, central-control-plane, or
   headless peer-mesh deployment.
2. Verify token scope: control-plane token, cluster token, and direct-mesh token
   should not be silently interchangeable unless documented.
3. Check whether the route/action is read, write, cluster-management, remote
   command execution, or direct TCP access.
4. Add or update tests for auth success, missing-token failure, and wrong-token
   failure when the behavior is externally reachable.
5. Keep docs/security.md and this module marker file synchronized.
