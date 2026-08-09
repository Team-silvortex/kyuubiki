# Operations Guide

This document collects the current operational modes and the most important
environment switches.

Use this page for runtime behavior:

- deployment modes
- discovery modes
- watchdog and security controls
- operator-facing runtime entrypoints

Use this page as the operational source, not as the primary owner of:

- product-role separation
- headless runtime contract layering
- single-orchestrator versus offline-mesh binding rules

Do not use this page as the main source for:

- build output locations
- packaging artifact layout
- step-by-step desktop release execution

Those belong to:

- [app-runtime-boundaries.md](app-runtime-boundaries.md)
- [headless-agent-contract.md](headless-agent-contract.md)
- [agent-control-authority.md](agent-control-authority.md)
- [packaging-and-deployment.md](packaging-and-deployment.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)

It now reflects the `moxi 2.x` product shape:

- `Hub` as the desktop operator shell
- `Workbench` as the focused modeling and analysis surface
- `Installer` as the heavier bootstrap and deployment surface
- `orchestrator/control plane` as one managed runtime target
- a broader FEM operator family spanning axial, thermal, spring, beam, torsion,
  truss, plane, and frame studies

## Main deployment modes

### Local workstation

- `KYUUBIKI_DEPLOYMENT_MODE=local`
- `KYUUBIKI_STORAGE_BACKEND=sqlite`
- local frontend + orchestrator + local Rust agents

### Cloud control plane

- `KYUUBIKI_DEPLOYMENT_MODE=cloud`
- `KYUUBIKI_STORAGE_BACKEND=postgres`
- frontend and orchestrator deployed centrally

### Distributed control plane

- `KYUUBIKI_DEPLOYMENT_MODE=distributed`
- `KYUUBIKI_AGENT_DISCOVERY=manifest|registry`
- remote solver agents register or are discovered from manifests

## Agent discovery modes

### Static

Use:

- `KYUUBIKI_AGENT_DISCOVERY=static`
- `KYUUBIKI_AGENT_ENDPOINTS=127.0.0.1:5001,127.0.0.1:5002`

### Manifest

Use:

- `KYUUBIKI_AGENT_DISCOVERY=manifest`
- `KYUUBIKI_AGENT_MANIFEST_PATH=/path/to/agents.json`

Schema:

- [agent-manifest.schema.json](../schemas/agent-manifest.schema.json)

### Registry

Use:

- `KYUUBIKI_AGENT_DISCOVERY=registry`

Remote agents can:

- register
- heartbeat
- unregister

## Watchdog controls

The orchestrator watchdog protects long-running jobs.

The main knobs cover:

- scan interval
- stale active-job heartbeat timeout
- queued dispatch timeout
- hard active-job timeout
- agent connect timeout
- agent receive timeout

The concrete watchdog variables are `KYUUBIKI_WATCHDOG_SCAN_INTERVAL_MS`,
`KYUUBIKI_WATCHDOG_STALE_JOB_MS`, `KYUUBIKI_WATCHDOG_QUEUE_TIMEOUT_MS`, and
`KYUUBIKI_WATCHDOG_JOB_TIMEOUT_MS`. Queued jobs are not judged by the active
heartbeat budget. Active jobs remain live while the Agent sends heartbeats,
including long result serialization and artifact upload phases.

The Rust Agent also owns a local stale-execution watchdog. Configure it with
`KYUUBIKI_AGENT_WATCHDOG_SCAN_INTERVAL_MS` and
`KYUUBIKI_AGENT_WATCHDOG_STALE_EXECUTION_MS`, or the matching
`--watchdog-scan-interval-ms` and `--watchdog-stale-execution-ms` flags. The
defaults are 5 seconds and 30 seconds. Setting either value to zero disables
the local scanner, and the effective policy is visible in the Agent watchdog
snapshot. Solver heartbeats refresh job-bound executions; an expired execution
releases its slot, records `watchdog_timeout`, requests cooperative
cancellation, and rejects a late result without duplicating the failure record.
This is not a process-level force-kill boundary.

Agent admission is capacity-gated for static, manifest, and registry discovery.
`KYUUBIKI_AGENT_QUEUE_TIMEOUT_MS` bounds the Orchestra-side capacity queue;
`KYUUBIKI_ARTIFACT_EXECUTION_TIMEOUT_MS` independently bounds large artifact-backed
solver execution. The default qualification posture gives each phase 30 minutes.
Do not use a Headless `job_wait.timeout_ms` value as a server execution policy:
it only controls how long that client polls. Every job response publishes
`status_detail.timing`, including the effective phase timeout, submission
deadline, execution start, and execution deadline. `/api/health` publishes the
current `agent_execution_gate` capacity and queue counts.

Long-running Headless templates may opt into `resume_policy=server_deadline`.
That policy reuses the original `job_id` across bounded polling windows and
stops at the earlier of the server deadline or `max_total_timeout_ms`; it never
re-submits a solve. Missing server timing fails closed instead of creating an
unbounded wait.

## Security controls

Security controls currently group into:

- API access tokens
- cluster registration allowlists and replay/fingerprint checks
- protected read surfaces
- direct-mesh enablement and endpoint/token policy

These can now be written from the installer GUI Setup panel. For the current
guardrails and exact switch names, use:

- [security.md](security.md)

## Useful entry points

Common operator-facing entry points:

- `make start-local`
- `make start-cloud`
- `make start-distributed`
- `make hot-local`
- `make hot-cloud`
- `make hot-distributed`
- `make status`
- `make stop`
- `make desktop-status PLATFORM=all`

Build, packaging, desktop-release, and benchmark command matrices are kept in:

- [packaging-and-deployment.md](packaging-and-deployment.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)
- [testing-and-ci.md](testing-and-ci.md)

For operator-facing desktop control, Hub now mirrors several of these flows:

- desktop readiness and selected runtime-oriented staging visibility
- local / cloud / distributed hot-reload loop control
- runtime watch for stack and hot-loop logs
- local workload-catalog sync against the control plane

## Health and descriptors

Runtime visibility:

- `/api/health`
- `/api/v1/protocol`
- `/api/v1/protocol/agents`
- `/api/v1/agents`

## Remote pilot route

For the first Ubuntu-host rollout path, use:

- [remote-pilot.md](remote-pilot.md)

That page owns the staged rollout sketch for:

- first remote solver node
- remote control plane follow-up
- remote workload-source validation

Keep this operations guide focused on runtime modes, discovery knobs, and
operator entrypoints.
