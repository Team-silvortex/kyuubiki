# Operations Guide

This document collects the current operational modes and the most important
environment switches.

Use this page for runtime behavior:

- deployment modes
- discovery modes
- watchdog and security controls
- operator-facing runtime entrypoints

Do not use this page as the main source for:

- build output locations
- packaging artifact layout
- step-by-step desktop release execution

Those belong to:

- [packaging-and-deployment.md](packaging-and-deployment.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)

It now reflects the `tamamono 1.x` product shape:

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
- stale-job timeout
- hard job timeout
- agent connect timeout
- agent receive timeout

The concrete variable names remain the `KYUUBIKI_WATCHDOG_*` and
`KYUUBIKI_AGENT_*` controls already used by the runtime.

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

## Remote Pilot

Use this path when you want one `Ubuntu 24` machine to become the first real
remote target for `Kyuubiki Hub`.

Treat the rollout as three confidence levels:

1. `remote solver node`
   Keep the orchestrator local and run only a headless Rust agent on Ubuntu.
2. `remote control plane`
   Run frontend + orchestrator on Ubuntu and point Hub at it as a remote
   runtime target.
3. `remote catalog / workload source`
   Let Ubuntu also serve workload-catalog and project-bundle endpoints.

For the first practical operator pass, level `1` is the best return.

### Level 1: Remote solver node

Recommended shape:

- laptop:
  - Hub
  - Workbench
  - local orchestrator on `4000`
- Ubuntu host:
  - `kyuubiki-cli` Rust agent on `5001`

Minimum Ubuntu setup:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev screen curl git
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
git clone <your-repo-url> ~/kyuubiki
cd ~/kyuubiki
./scripts/kyuubiki build-agent
./scripts/kyuubiki agent -- --port 5001
```

If you want the agent to survive your shell session:

```bash
cd ~/kyuubiki
screen -S kyuubiki-agent-5001 -X quit >/dev/null 2>&1 || true
screen -dmS kyuubiki-agent-5001 sh -lc './scripts/kyuubiki agent -- --port 5001'
```

Laptop-side local orchestrator changes:

```bash
export KYUUBIKI_AGENT_DISCOVERY=static
export KYUUBIKI_AGENT_ENDPOINTS=192.168.1.50:5001
./scripts/kyuubiki restart-local
```

If you want to persist those values, put them in `.env.local` at the root of
the laptop checkout.

Validate before using Hub:

```bash
curl http://127.0.0.1:4000/api/v1/protocol/agents
./scripts/kyuubiki status
curl http://127.0.0.1:4000/api/health
```

Best first studies:

- `axial_bar_1d`
- `beam_1d`
- `thermal_bar_1d`

### Level 2: Remote control plane

Recommended shape:

- laptop:
  - Hub
- Ubuntu host:
  - frontend
  - orchestrator
  - one or more Rust agents

Validate:

- Hub can treat Ubuntu as the active runtime target
- runtime watch still makes failures understandable
- Workbench can open the remote frontend and submit a study
- token-protected reads still behave correctly

### Level 3: Remote workload source

Recommended shape:

- Ubuntu serves:
  - `/api/v1/workloads/catalog`
  - `/api/v1/projects/:project_id/bundle`

Validate:

- Hub classifies remote workloads correctly
- provenance still reads as first-party remote control plane
- `Mechanical / Thermal / Thermo-mechanical` filters still work
- opening a remote workload lands in the expected Workbench context

### Good enough criteria

Treat the remote pilot as successful when:

- Hub can distinguish local vs remote runtime context without confusion
- one remote agent can solve representative studies reliably
- first-line troubleshooting is still possible from Hub alone
- remote control-plane reads feel no rougher than local reads
- bundle/workload provenance remains understandable to a non-author
