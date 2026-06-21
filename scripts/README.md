# Scripts

This directory contains host-native operational entry points.

- `kyuubiki`
  Unified launcher for local, cloud, and distributed development flows.
- `kyuubiki-lab`
  Thin operational wrapper for the shared Ubuntu lab machine that now hosts
  the standard download/deploy server plus the shared solver-agent test node.
- `create-release-snapshot.mjs`
  Scaffold a new lightweight release snapshot manifest and update the release
  index. When a snapshot is marked `current`, it also advances the shared
  shipping-version contracts.
- `build-update-catalog.mjs`
  Generate the shared update catalog JSON plus HTML docs from release snapshots
  and the human-owned channel contract.
- `build-installation-integrity-docs.mjs`
  Generate the installation integrity HTML docs for both the repository-level
  book and the Hub-facing mirror shelf.
- `upload-desktop-release-remote.sh`
  Upload generated desktop bundles, staged `dist/` outputs, and release
  metadata to a remote download server, with an optional post-upload local
  cleanup path for disk-constrained workstations.
- `audit-version-line.mjs`
  Audit repository-wide version contracts and inventory visible version
  references before advancing a shipping line such as `tamamono 1.7.0`.
- `check-doc-book.mjs`
  Verify the centralized docs book and Hub mirrors for version alignment,
  broken local links, required chapter markers, and old legacy wording.
- `sync-doc-book-version.mjs`
  Update the hand-maintained book entry pages to the current shipping version
  without touching the generated installation or update-catalog pages.
- `release-metadata.mjs`
  Shared release-path, JSON, artifact, and shipping-version helpers used by the
  release and installation-doc generators.

Use this directory for operator-facing workflow wrappers, not for source
libraries or generated output.

Typical responsibilities:

- start/stop/restart orchestration
- hot-reload/watch orchestration for local development
- mode switching (`local`, `cloud`, `distributed`)
- verification/test wrappers
- component-scoped build entry points
- runtime and desktop packaging entry points
- installer entry points
- release snapshot scaffolding
- unified update-catalog generation
- release metadata normalization across `releases/` and `deploy/`
- remote release artifact upload and local bundle cleanup

Useful smoke wrappers:

- `./scripts/kyuubiki smoke`
  Current Elixir -> Rust integration smoke flow.
- `./scripts/kyuubiki sdk-smoke`
  Python / Elixir / Rust headless SDK smoke suite.
- `./scripts/kyuubiki frontend-test`
  Frontend typecheck plus production build verification.
- `./scripts/kyuubiki workflow-preflight`
  Workflow topology plus search/layout guard suite. Start `npm run dev` under
  `apps/frontend` in a separate shell first because the browser-backed checks
  exercise the live workbench benchmark surface.
- `./scripts/kyuubiki desktop-upload-remote macos`
  Upload the current shipping-version desktop release outputs to the remote
  download server. Override the target with
  `KYUUBIKI_RELEASE_REMOTE_HOST=user@host`, optionally provide
  `KYUUBIKI_RELEASE_REMOTE_PASSWORD=...` for `sshpass`-backed non-interactive
  auth, and set `PURGE_LOCAL=1` to remove local `dist/` and Tauri bundle
  outputs after a successful upload.
- `./scripts/run-direct-mesh-benchmark-container.sh --repeat 3`
  Build the dedicated Docker harness, run the direct-mesh integration suite
  multiple times, and write JSON plus Markdown summaries under
  `tmp/direct-mesh-benchmark-container/`. For LAN agent discovery, prefer
  `DOCKER_RUN_NETWORK=host`. The current checked-in baseline snapshot is
  `tests/integration/benchmarks/direct-mesh-docker-baseline.json`.
- `node ./scripts/compare-direct-mesh-benchmark.mjs --current tmp/direct-mesh-benchmark-container/latest/summary.json --baseline tests/integration/benchmarks/direct-mesh-docker-baseline.json --report-out tmp/direct-mesh-benchmark-container/latest/compare.md --json-out tmp/direct-mesh-benchmark-container/latest/compare.json`
  Compare a direct-mesh Docker benchmark summary against the checked-in
  baseline and emit both Markdown and machine-readable diff artifacts.
- `./scripts/run-direct-mesh-benchmark-regression.sh`
  Run the remote direct-mesh Docker benchmark on `kyuubiki-lab`, copy the
  resulting summary back into the local workspace, and compare it against the
  checked-in baseline with regression thresholds. This wrapper expects
  passwordless `sudo` on the remote lab host.
- `cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_large_graph_report_test.exs`
  Runs the orchestrated large-graph workflow benchmark suite and writes a
  machine-readable JSON report with per-case performance summaries at
  `../tmp/workflow-large-graph-benchmark.json` from `apps/web`, which is the
  repository-level `tmp/workflow-large-graph-benchmark.json`.
- `cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs`
  Runs the catalog-backed composite workflow benchmark suite for the current
  thermal and guarded coupled flows across the default 8-case quad/triangle
  suite, then writes a machine-readable JSON report at
  `../tmp/workflow-catalog-benchmark.json` from `apps/web`, which is the
  repository-level `tmp/workflow-catalog-benchmark.json`. The current checked-in
  baseline snapshot is
  `tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json`.
- `node ./scripts/compare-workflow-catalog-benchmark.mjs --current tmp/workflow-catalog-benchmark.json --baseline tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json --report-out tmp/workflow-catalog-benchmark.compare.md --json-out tmp/workflow-catalog-benchmark.compare.json`
  Compare a workflow catalog benchmark report against the checked-in 8-case
  baseline and emit both Markdown and machine-readable diff artifacts.
- `make test-integration-workflow-catalog-compare CURRENT=tmp/workflow-catalog-benchmark.json`
  Makefile entry for comparing a workflow catalog benchmark report against the
  checked-in 8-case baseline.
- `make test-integration-workflow-catalog-report`
  Run the local workflow catalog benchmark report case and compare it against
  the checked-in baseline.
- `./scripts/run-workflow-catalog-benchmark-regression.sh`
  Run the remote workflow catalog benchmark on `kyuubiki-lab`, copy the
  resulting summary back into the local workspace, and compare it against the
  checked-in baseline with per-case regression thresholds.
- `./scripts/run-workflow-mesh-regression.sh`
  Run the current three-test distributed workflow mesh regression trio in
  strict sequence on the local machine so the shared local orchestrator port
  does not collide across tests. This emits `run.log`, `summary.json`, and
  `README.md` under `tmp/workflow-mesh-regression/<slug>/`.
- `./scripts/run-workflow-mesh-regression-remote.sh`
  Sync the mesh workflow regression wrappers plus integration tests to
  `kyuubiki-lab`, run the distributed workflow mesh regression trio there,
  and pull the combined run log plus summary artifacts back into
  `tmp/workflow-mesh-regression/`.
- `./scripts/build-workflow-mesh-regression-summary.mjs --log tmp/workflow-mesh-regression/<slug>/run.log --output-dir tmp/workflow-mesh-regression/<slug>`
  Rebuild the machine-readable and human-readable summary artifacts for a
  workflow mesh regression run from the captured TAP log.
- `./scripts/build-workflow-mesh-regression-index.mjs --root tmp/workflow-mesh-regression`
  Rebuild the retained workflow mesh run index and emit `index.json`,
  `README.md`, and `index.html` under `tmp/workflow-mesh-regression/`.
- `make test-integration-workflow-mesh`
  Makefile entry for the local distributed workflow mesh regression trio.
- `make test-integration-workflow-mesh-nightly`
  Makefile entry for the remote `kyuubiki-lab` workflow mesh regression flow.
- `make test-integration-workflow-catalog-nightly`
  Makefile entry for the remote workflow catalog regression flow against the
  checked-in baseline.
- `./scripts/run-standard-benchmark-regression.sh`
  Sync benchmark-only source to `kyuubiki-lab`, run the standard Rust
  benchmark regression trio there, and pull the merged/per-matrix comparison
  reports back into `tmp/standard-benchmark/`. The wrapper also refreshes
  `tmp/standard-benchmark/index.json` plus `README.md` and prunes old run
  folders by retention count.
- `./scripts/build-standard-benchmark-index.mjs`
  Rebuild the local standard benchmark run index under `tmp/standard-benchmark/`
  and prune older run directories outside the retained window. This emits
  `index.json`, `README.md`, and `index.html`.
- `./scripts/build-nightly-artifact-overview.mjs`
  Rebuild the top-level `tmp/` nightly artifact overview across the direct-mesh,
  workflow-catalog, and standard-benchmark lanes. This emits `tmp/README.md`,
  `tmp/nightly-overview.json`, and `tmp/nightly-overview.html`.
- `./scripts/build-regression-lane-catalog.mjs --tmp-root tmp`
  Rebuild the normalized cross-lane regression catalog for the latest retained
  direct-mesh, workflow-catalog, and workflow-mesh outputs. This emits
  `tmp/regression-lane-catalog.json`, `tmp/regression-lane-catalog.md`, and
  `tmp/regression-lane-catalog.html`, including a shared `gate` decision layer
  with per-lane reasons plus the catalog-level `overall_gate_status`.
- `./scripts/build-regression-gate-report.mjs --tmp-root tmp`
  Collapse the shared regression lane catalog into a CI/installer-friendly gate
  output. This emits `tmp/regression-gate-report.json` plus
  `tmp/regression-gate-report.md`, prints the overall gate status, exits `2`
  for `fail`, and can exit non-zero for `warn` via `--fail-on-warn`.
- `make benchmark-standard-nightly`
  Makefile entry for the remote standard benchmark regression flow against the
  checked-in standard baselines.
- `.github/workflows/standard-benchmark-nightly.yml`
  Self-hosted GitHub Actions entry for the remote standard benchmark
  regression flow and artifact upload path.
- `.github/workflows/workflow-mesh-nightly.yml`
  Self-hosted GitHub Actions entry for the remote workflow mesh regression
  flow, unified gate refresh, and artifact upload path.
- `.github/workflows/workflow-catalog-nightly.yml`
  Self-hosted GitHub Actions entry for the remote workflow catalog regression
  flow and artifact upload path.
- `.github/workflows/direct-mesh-docker-nightly.yml`
  Self-hosted GitHub Actions entry for the remote direct-mesh Docker
  regression flow, unified gate refresh, and artifact upload path.
- `cd apps/web && mix test test/kyuubiki_web/api/workflow_large_graph_api_test.exs && ELIXIR_PA="$(find "$PWD/_build/test/lib" -maxdepth 2 -type d -name ebin -print | tr '\n' ' ')" elixir -pa $ELIXIR_PA ../../scripts/workflow-large-graph-benchmark.exs 96 256 512 --output ../../tmp/workflow-large-graph-benchmark.json`
  Lower-level host script path for environments that allow plain Elixir TCP
  sockets outside `mix test`.
- `cd apps/web && mix test test/kyuubiki_web/api/workflow_catalog_thermal_job_api_test.exs test/kyuubiki_web/api/workflow_catalog_guard_job_api_test.exs && ELIXIR_PA="$(find "$PWD/_build/test/lib" -maxdepth 2 -type d -name ebin -print | tr '\n' ' ')" elixir -pa $ELIXIR_PA ../../scripts/workflow-catalog-benchmark.exs --repeat 5 --output ../../tmp/workflow-catalog-benchmark.json`
  Lower-level host script path for replaying the catalog-backed composite
  workflow benchmark without going through a dedicated `mix test` report case.

Examples now include:

- `hot-local`
- `hot-cloud`
- `hot-distributed`
- `hot-web`
- `hot-agent`
- `hot-hub-gui`
- `hot-installer-gui`
- `hot-workbench-gui`
- `build-frontend`
- `build-orchestrator`
- `build-agent`
- `build-hub-gui`
- `build-installer-gui`
- `build-workbench-gui`
- `package-runtime`
- `package-desktop`
- `desktop-upload-remote`
- `desktop-status`
- `desktop-stage`
- `desktop-build-host`
- `desktop-release`
- `desktop-verify`
- `sync-desktop-shared`
- `test-hub-gui`
- `test-installer-gui`
- `test-workbench-gui`

Keep these scripts thin. Product logic should live in the application/runtime
code, not in shell branching.

Hot-reload note:

- Next.js and Tauri already provide their own dev/HMR loops.
- `./scripts/kyuubiki hot-*` adds the missing restart-on-change layer for the
  non-Phoenix Elixir control plane and Rust solver agents so the whole stack
  can iterate under one operator command.
