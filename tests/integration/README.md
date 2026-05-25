# Integration Tests

This directory holds cross-process integration tests that exercise multiple
Kyuubiki programs working together.

The first target focuses on the local workstation stack:

- unified launcher
- orchestrator API
- Rust solver agents
- real HTTP job submission and polling

Current orchestrated API smoke now includes:

- `axial_bar_1d`
- `frame_3d`
- `thermal_frame_3d`
- `thermal_truss_3d`

Run with:

- `make test-integration`
- `make test-integration-api`
- `make test-integration-cluster`
- `make test-integration-direct-mesh`
- `make test-integration-ui-mechanical`
- `make test-integration-ui-thermal`

The Workbench UI smoke suite is split by domain so failures are easier to triage:

- `workbench-ui-mechanical-smoke.test.mjs`
- `workbench-ui-thermal-smoke.test.mjs`
