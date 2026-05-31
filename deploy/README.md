# Deploy Assets

This directory holds deployment-time descriptors rather than source code.

- `agents.local.json`
  Example local agent manifest for workstation runs.
- `agents.distributed.example.json`
  Example distributed manifest for a remote solver cluster.
- `workload-catalog.example.json`
  Example workload catalog for Hub library import, remote catalog sync, and
  installer/bootstrap flows.

These files are consumed by runtime and operator-facing surfaces as deployment
descriptors:

- the orchestrator reads agent-manifest topology
- the installer uses the same descriptors when bootstrapping layouts or remote
  targets
- Hub library workflows can import workload catalogs from the example shape

These files are safe to version because they describe topology and catalog
shape rather than live runtime state.

Use the shared schema references when editing or extending these examples:

- `/Users/Shared/chroot/dev/kyuubiki/schemas/agent-manifest.schema.json`
- `/Users/Shared/chroot/dev/kyuubiki/schemas/workload-catalog.schema.json`
