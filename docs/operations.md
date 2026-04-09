# Operations Guide

This document collects the current operational modes and the most important
environment switches.

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

- [agent-manifest.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/agent-manifest.schema.json)

### Registry

Use:

- `KYUUBIKI_AGENT_DISCOVERY=registry`

Remote agents can:

- register
- heartbeat
- unregister

## Watchdog controls

The orchestrator watchdog protects long-running jobs.

Environment variables:

- `KYUUBIKI_WATCHDOG_SCAN_INTERVAL_MS`
- `KYUUBIKI_WATCHDOG_STALE_JOB_MS`
- `KYUUBIKI_WATCHDOG_JOB_TIMEOUT_MS`
- `KYUUBIKI_AGENT_CONNECT_TIMEOUT_MS`
- `KYUUBIKI_AGENT_RECV_TIMEOUT_MS`

## Security controls

Common environment switches:

- `KYUUBIKI_API_TOKEN`
- `KYUUBIKI_CLUSTER_API_TOKEN`
- `KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS`
- `KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS`
- `KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS`
- `KYUUBIKI_PROTECT_READS=true|false`
- `KYUUBIKI_DIRECT_MESH_ENABLED=true|false`
- `KYUUBIKI_DIRECT_MESH_TOKEN`

These can now be written from the installer GUI Setup panel.

## Useful entry points

- `make start-local`
- `make start-cloud`
- `make start-distributed`
- `make status`
- `make stop`
- `make benchmark-compare PROFILE=medium`
- `make benchmark-report PROFILE=10k`

## Health and descriptors

Runtime visibility:

- `/api/health`
- `/api/v1/protocol`
- `/api/v1/protocol/agents`
- `/api/v1/agents`
