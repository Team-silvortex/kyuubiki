# Benchmark Scripts

Focused reference for benchmark and regression script entry points.

- `cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_large_graph_report_test.exs`
  runs the orchestrated large-graph workflow benchmark suite and writes
  `tmp/workflow-large-graph-benchmark.json`.
- `cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs`
  runs the catalog-backed composite workflow benchmark suite and writes
  `tmp/workflow-catalog-benchmark.json`.
- `./scripts/kyuubiki compare-workflow-catalog-benchmark --current tmp/workflow-catalog-benchmark.json --baseline tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json --report-out tmp/workflow-catalog-benchmark.compare.md --json-out tmp/workflow-catalog-benchmark.compare.json`
  compares a workflow catalog benchmark report against the checked-in baseline.
- `make test-integration-workflow-catalog-compare CURRENT=tmp/workflow-catalog-benchmark.json`
  compares a workflow catalog benchmark report through Make.
- `make test-integration-workflow-catalog-report`
  runs the local workflow catalog benchmark report case and compares it against
  the checked-in baseline.
- `./scripts/run-workflow-catalog-benchmark-regression.sh`
  compatibility shim for `./scripts/kyuubiki workflow-catalog-benchmark-regression`.
  It runs the remote workflow catalog benchmark on `kyuubiki-lab` and compares
  the result against the checked-in baseline.
- `./scripts/kyuubiki workflow-mesh-regression`
  native local workflow mesh regression runner. It emits `run.log`,
  `summary.json`, and `README.md` under `tmp/workflow-mesh-regression/<slug>/`.
- `./scripts/run-workflow-mesh-regression.sh`
  compatibility shim for the native workflow mesh command.
- `./scripts/run-workflow-mesh-regression-remote.sh`
  compatibility shim for `./scripts/kyuubiki workflow-mesh-regression-remote`.
  It syncs the mesh workflow regression tests to `kyuubiki-lab`, runs them
  there, and pulls summary artifacts back.
- `./scripts/kyuubiki build-workflow-mesh-regression-summary --log tmp/workflow-mesh-regression/<slug>/run.log --output-dir tmp/workflow-mesh-regression/<slug>`
  rebuilds summary artifacts from a captured workflow mesh TAP log.
- `./scripts/kyuubiki build-workflow-mesh-regression-index --root tmp/workflow-mesh-regression`
  rebuilds the retained workflow mesh index.
- `make test-integration-workflow-mesh`
  local distributed workflow mesh regression trio.
- `make test-integration-workflow-mesh-nightly`
  remote `kyuubiki-lab` workflow mesh regression flow.
- `make test-integration-workflow-catalog-nightly`
  remote workflow catalog regression flow.
- `PROFILE=400k MATRIX=mechanical-core CASE=axial-bar-400k REPEAT=1 ./scripts/run-benchmark-profile-remote.sh`
  compatibility shim for `./scripts/kyuubiki benchmark-profile-remote`. It runs
  one remote Rust benchmark profile/matrix smoke without requiring a checked
  baseline and writes artifacts under `tmp/benchmark-profile/`.
- `PROFILE=500k MATRIX=mechanical-core CASE=axial-bar-500k REPEAT=1 ./scripts/run-benchmark-profile-remote.sh`
  first 500k remote probe. Treat 500k as remote-first exploratory evidence:
  validate narrow cases before promoting any full matrix into a scheduled lane.
- `make benchmark-profile-plan PROFILE=500k`
  prints the case-by-case 500k remote probe plan from
  `config/benchmark-profile-coverage.json` without running it. Add
  `MATRIX=<matrix>`, `CASE=<substring>`, or `LIMIT=<n>` to narrow the plan.
- `SHAPES=1 make benchmark-profile-plan PROFILE=500k MATRIX=thermal-structural LIMIT=2`
  includes generated nodes, elements, and DOFs beside each planned remote probe
  without running the solver.
- `FORMAT=json SHAPES=1 make benchmark-profile-plan PROFILE=500k MATRIX=thermal-structural LIMIT=2`
  emits the same dry-run plan as machine-readable JSON for dashboards, CI
  artifacts, or batch controllers. JSON mode is intentionally dry-run-only.
- `PLAN_OUT=tmp/benchmark-profile-plan.json SHAPES=1 make benchmark-profile-plan PROFILE=500k MATRIX=thermal-structural LIMIT=2`
  writes the structured plan to a repo-local JSON file while keeping the normal
  human-readable plan on stdout.
- `make benchmark-shapes PROFILE=500k MATRIX=thermal-structural FORMAT=json`
  prints nodes, elements, and DOFs for generated benchmark cases without
  solving them. Use this before launching a remote batch when catalog scale is
  the thing being checked.
- `make benchmark-profile-plan PROFILE=500k LIMIT=2 EXECUTE=1`
  executes the first two selected remote probes sequentially. Each case receives
  its own `OUTPUT_SLUG`, so retained profile summaries do not overwrite each
  other.
- `./scripts/build-benchmark-profile-index.mjs`
  rebuilds the exploratory benchmark profile index from retained
  `summary.json` files under `tmp/benchmark-profile/`. Coverage summaries
  track 400k and 500k targets across `mechanical-core`, `thermal-core`,
  `compound-core`, and `thermal-structural`.
- `./scripts/run-standard-benchmark-regression.sh`
  compatibility shim for `./scripts/kyuubiki standard-benchmark-regression`.
  It syncs Rust sources to `kyuubiki-lab`, runs the standard Rust benchmark
  regression trio, pulls reports back into `tmp/standard-benchmark/`, refreshes
  index artifacts, and prunes old run folders.
- `make benchmark-compare MATRIX=<matrix> PROFILE=<profile> CASE=<substring> REPEAT=1`
  local hot-case comparison before using the remote lane. Case-filtered
  baseline/report targets use case-suffixed filenames to avoid overwriting
  full-matrix artifacts.
- `./scripts/kyuubiki build-standard-benchmark-index`
  rebuilds the local standard benchmark index under `tmp/standard-benchmark/`.
- `./scripts/build-nightly-artifact-overview.mjs`
  rebuilds the top-level `tmp/` nightly artifact overview.
- `make benchmark-standard-nightly`
  Makefile entry for the remote standard benchmark regression flow.
- `.github/workflows/standard-benchmark-nightly.yml`
  self-hosted GitHub Actions entry for the remote standard benchmark lane.
- `.github/workflows/workflow-mesh-nightly.yml`
  self-hosted GitHub Actions entry for workflow mesh regression.
- `.github/workflows/workflow-catalog-nightly.yml`
  self-hosted GitHub Actions entry for workflow catalog regression.
- `.github/workflows/direct-mesh-docker-nightly.yml`
  self-hosted GitHub Actions entry for direct-mesh Docker regression.
