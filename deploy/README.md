# Deploy Assets

This directory holds deployment-time descriptors rather than source code.

- `agents.local.json`
  Example local agent manifest for workstation runs.
- `agents.distributed.example.json`
  Example distributed manifest for a remote solver cluster.

These files are consumed by the orchestrator and installer as deployment
descriptors. They are safe to version because they describe topology rather than
runtime state.
