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
- `thermal_bar_1d`
- `spring_1d`
- `spring_2d`
- `spring_3d`
- `thermal_beam_1d`
- `torsion_1d`
- `heat_bar_1d`
- `heat_plane_triangle_2d`
- `heat_plane_quad_2d`
- `frame_2d`
- `frame_3d`
- `truss_2d`
- `plane_triangle_2d`
- `truss_3d`
- `plane_quad_2d`
- `thermal_frame_2d`
- `thermal_plane_quad_2d`
- `thermal_plane_triangle_2d`
- `thermal_truss_2d`
- `thermal_frame_3d`
- `thermal_truss_3d`

The orchestrator API smoke entrypoint is intentionally small:

- `orchestrator-agent-api-smoke.test.mjs`
- `orchestrator-agent-api-smoke/axial-thermal-smoke.test.mjs`
- `orchestrator-agent-api-smoke/structural-smoke.test.mjs`
- `orchestrator-agent-api-smoke/frame-thermal-smoke.test.mjs`
- `orchestrator-agent-api-smoke/workflow-smoke.test.mjs`

Run with:

- `make test-integration`
- `make test-integration-api`
- `make test-integration-cluster`
- `make test-integration-direct-mesh`
- `make test-integration-benchmark-profile-index`
- `make test-integration-direct-mesh-docker`
- `make test-integration-remote-ssh-fixture`
- `make test-integration-direct-mesh-docker-compare`
- `make test-integration-direct-mesh-docker-report`
- `make test-integration-ui-mechanical`
- `make test-integration-ui-thermal`

Remote deployment SSH fixture scaffolding lives under:

- `tests/integration/remote-ssh-fixture/`

That fixture is manual-only for now. It binds `127.0.0.1:2222`, uses a
throwaway key under `tests/integration/remote-ssh-fixture/runtime/`, and is not
started by the default integration targets. Use it explicitly with:

- `make test-integration-remote-ssh-fixture`

Before executable runs, inspect the contracts with:

- `cargo run -p kyuubiki-installer -- remote-ssh-fixture`
- `cargo run -p kyuubiki-installer -- remote-ssh-fixture-plan`

For repeatable host-independent mesh baselines, prefer the Docker harness:

- `make test-integration-direct-mesh-docker REPEAT=3`

That path builds the dedicated benchmark image, runs the direct-mesh suite
inside the container, and exports machine-readable summaries under
`tmp/direct-mesh-benchmark-container/`. The Make target defaults
`DOCKER_RUN_NETWORK=host` so the container can discover LAN direct-mesh agents.

The current checked-in Docker baseline lives at:

- `tests/integration/benchmarks/direct-mesh-docker-baseline.json`

The current checked-in workflow catalog baseline lives at:

- `tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json`

The benchmark profile index contract smoke is intentionally local-only. It
creates temporary retained-run summaries and verifies that the profile index
advisory gate reports `pass` for valid metrics, `warn` for an empty root, and
keeps indexing when a malformed retained summary is skipped. It also covers the
default benchmark coverage manifest plus a custom `--coverage-targets`
manifest so future physics matrices can add completeness checks without
changing the indexer code:

- `make test-integration-benchmark-profile-index`

To compare an existing workflow catalog benchmark report against that baseline:

- `node ./scripts/compare-workflow-catalog-benchmark.mjs --current tmp/workflow-catalog-benchmark.json --baseline tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json --report-out tmp/workflow-catalog-benchmark.compare.md --json-out tmp/workflow-catalog-benchmark.compare.json`
- `make test-integration-workflow-catalog-compare CURRENT=tmp/workflow-catalog-benchmark.json`

To run a fresh workflow catalog benchmark and emit current-vs-baseline reports:

- `make test-integration-workflow-catalog-report`

To run the remote workflow catalog regression flow against `kyuubiki-lab`:

- `./scripts/run-workflow-catalog-benchmark-regression.sh`
- `make test-integration-workflow-catalog-nightly`

To compare an existing Docker summary against that baseline:

- `make test-integration-direct-mesh-docker-compare CURRENT=tmp/direct-mesh-benchmark-container/latest/summary.json`

To run a fresh repeat-3 Docker benchmark and emit current-vs-baseline reports:

- `make test-integration-direct-mesh-docker-report REPEAT=3`

To run the remote regression flow against `kyuubiki-lab` and fail on threshold
regressions:

- `make test-integration-direct-mesh-docker-nightly`

The repository also includes `.github/workflows/direct-mesh-docker-nightly.yml`
for self-hosted runners on the same LAN. It is gated behind
`vars.KYUUBIKI_DIRECT_MESH_SELF_HOSTED == 'true'` so public GitHub runners do
not try to reach the private lab machine. The remote regression wrapper also
expects passwordless `sudo` on the lab host for the benchmark command path.

The repository also includes `.github/workflows/workflow-catalog-nightly.yml`
for self-hosted runners on the same LAN. It is gated behind
`vars.KYUUBIKI_WORKFLOW_CATALOG_SELF_HOSTED == 'true'` so public GitHub runners
do not try to reach the private lab machine.

The Workbench UI smoke suite is split by domain so failures are easier to triage:

- `workbench-ui-mechanical-smoke.test.mjs`
- `workbench-ui-thermal-smoke.test.mjs`

These smoke tests intentionally target the product-owned UI automation
contract instead of localized labels. Before changing Workbench rail buttons,
library tabs, sample cards, loaded-model state, runtime panels, viewport
panels, or control-mode windows, run `make check-ui-automation-contract`.
