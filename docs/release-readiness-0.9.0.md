# Release Readiness: v0.9.0

Use this checklist to decide whether Kyuubiki is ready for an initial usable
release.

This is intentionally split into three levels:

- `must`
  items that should be true before calling the release broadly usable
- `should`
  items that materially improve first-use and supportability
- `later`
  items worth doing, but not required for the first usable cut

## Must

### Solver-family scope is explicit

- [x] Define the official first-release study family list
- [x] Mark which studies are `fully supported` versus `experimental/minimal`
- [x] Keep that same support boundary consistent across docs, samples, and Hub

Suggested first-release family:

- `axial_bar_1d`
- `thermal_bar_1d`
- `spring_1d`
- `spring_2d`
- `spring_3d`
- `beam_1d`
- `torsion_1d`
- `frame_2d`
- `truss_2d`
- `truss_3d`
- `plane_triangle_2d`
- `plane_quad_2d`

Official support matrix:

- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)

### Every first-release study has a usable path

For each officially supported study:

- [x] one short “how to try this study” note exists
- [x] direct-mesh path is either supported or explicitly documented as not part
- [x] sample exists
- [ ] sample opens in Workbench
- [ ] orchestrated run works
- [ ] result/report/export path is present
- [x] each study row is explicitly reviewed against the release coverage table
      of the first release

Release coverage table:

- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)
- [release-smoke-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-smoke-matrix-0.9.0.md)

Sequential workflow smoke:

- [x] supported thermal -> thermo-mechanical bridge workflows are listed and smoke-tracked

Static evidence for official sample presence:

- [apps/frontend/src/lib/models/sample-library.ts](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/lib/models/sample-library.ts)
- [apps/frontend/public/models](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/public/models)

### Hub is a real first-run entrypoint

- [ ] Hub can launch the common local path without hand-editing commands
- [ ] Hub exposes recent workloads/projects clearly enough for repeat use
- [ ] Hub can open Workbench with usable context
- [ ] Hub can sync or attach local workloads in a way that feels stable
- [x] Hub has a simple recommended first-run path

Suggested first-run path:

1. start local stack
2. sync local workload catalog
3. open a sample workload
4. launch into Workbench

Reference:

- [release-first-run-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-first-run-0.9.0.md)

### Release validation baseline exists

- [x] `make test-web`
- [x] `make test-rust`
- [x] `make test-frontend`
- [x] `make test-sdk`
- [x] `make test-integration`
- [x] `make test-hub-gui`
- [x] `make test-installer-gui`
- [x] `make test-workbench-gui`
- [x] `make desktop-status PLATFORM=all`

- [x] release owners agree which failures block release and which only warn

Latest confirmed integration baseline:

- `make test-integration` passed on `2026-05-20`
- the suite now includes:
  - API smoke
  - cluster smoke
  - direct-mesh smoke
  - Workbench UI smoke for representative `Mechanical` studies
  - Workbench UI smoke for representative `Thermal` and `Thermo-mechanical`
    studies
- detailed per-study evidence is tracked in
  [release-smoke-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-smoke-matrix-0.9.0.md)

Latest confirmed desktop baseline:

- `make test-hub-gui` passed on `2026-05-20`
- `make test-installer-gui` passed on `2026-05-20`
- `make test-workbench-gui` passed on `2026-05-20`
- `make desktop-status PLATFORM=all` passed on `2026-05-20`
- desktop status currently reports:
  - macOS, Linux, and Windows scaffolds are present
  - manifests and icon inputs are ready across all three desktop shells
  - macOS host bundles are not yet built on this machine

Latest confirmed app/runtime baseline:

- `make test-frontend` passed on `2026-05-20`
  - `typecheck` passed
  - `next build` passed
- `make test-web` passed on `2026-05-20`
  - ExUnit result: `74 tests, 0 failures`
  - note: this suite required running outside the filesystem sandbox because
    Mix local PubSub opens a TCP socket during test startup
- `make test-rust` passed on `2026-05-20`
  - Rust workspace tests passed across protocol, solver, engine, CLI,
    installer, desktop runtime, and benchmark crates
- `make test-sdk` passed on `2026-05-20`
  - Python SDK smoke passed
  - Elixir SDK smoke passed: `1 test, 0 failures`
  - Rust SDK smoke passed: `1 passed; 0 failed`
  - note: this suite required running outside the filesystem sandbox because
    the Python SDK smoke binds a localhost test server

Agreed `v0.9.0` release gate:

- `blockers`
  - any failure in the validation baseline above
  - any failure in an officially supported study row that removes
    `sample opens`, `orchestrated run`, `report usable`, or `export usable`
  - any regression that breaks the documented `heat -> thermo-mechanical`
    bridge workflows
  - missing or broken desktop manifests, icon inputs, or runtime scaffolds for
    a supported platform target
- `warnings`
  - missing host bundle artifacts on a machine that has not yet run
    `desktop-build-host`
  - gaps in `minimal` study families that do not contradict the support matrix
  - UI polish, copy, or layout issues that do not block first-run paths
  - non-blocking diagnostic or observability rough edges when logs and status
    remain discoverable

### Failure and support path is minimally usable

- [x] runtime logs are discoverable
- [x] Hub/runtime watch is good enough for first-line troubleshooting
- [x] common failure messages are understandable enough for non-authors
- [x] a short “where to look when things fail” operator note exists

Reference:

- [release-troubleshooting-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-troubleshooting-0.9.0.md)
- Hub evidence:
  - [apps/hub-gui/ui/index.html](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/ui/index.html)
  - [apps/hub-gui/ui/app.js](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/ui/app.js)
  - [apps/hub-gui/README.md](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/README.md)
- latest operator-facing copy pass:
  - runtime log read failures now explain what to check next
  - desktop status failures now point back to local runtime tools
  - workload, assistant, and import failures now use action-oriented wording
  - Hub smoke still passes after the wording change

## Should

### Workbench onboarding is clearer

- [ ] a first-use path exists inside Hub or Workbench
- [ ] each solver family has one “official” sample to start from
- [ ] studies that are not full-editing parity say so clearly in the UI

### Release-facing docs feel coherent

- [ ] release summary
- [ ] full release notes
- [ ] GitHub Release version
- [ ] readiness checklist
- [ ] packaging/release docs reflect the same current product shape

### Operator workflow feels repeatable

- [ ] recent workload/project paths feel stable
- [ ] recent runtime/log paths in Hub are useful during real iteration
- [ ] desktop release flow is understandable without reading source

### Security guardrails are explained, not only implemented

- [ ] assistant/model endpoint constraints are documented
- [ ] security-event sources are documented
- [ ] sanitized log-copy behavior is visible in the UI
- [ ] high-risk assistant actions clearly require confirmation

## Later

### Deeper family completeness

- [ ] richer editing parity for every line-study family
- [ ] fuller result visualization for newer operator families
- [ ] more specialized object trees / result tables by study family

### Richer Hub productization

- [ ] stronger recent-project home screen
- [ ] more polished workload browsing from central catalogs
- [ ] richer guided flows and task templates

### Release automation polish

- [ ] more explicit release artifact generation from Hub
- [ ] more formal changelog automation
- [ ] stronger cross-platform build/report handoff

## Exit rule

Treat `v0.9.0` as an initial usable release candidate when:

- every `must` item above is either checked or consciously waived
- any waiver is written down with scope and owner
- the supported study family list is explicit
- Hub, Workbench, control plane, and agents can be exercised through one
  coherent first-run path

## Related docs

- [release-github-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-github-0.9.0.md)
- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)
- [release-smoke-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-smoke-matrix-0.9.0.md)
- [release-first-run-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-first-run-0.9.0.md)
- [release-troubleshooting-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-troubleshooting-0.9.0.md)
- [release-summary-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-summary-0.9.0.md)
- [release-notes-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-notes-0.9.0.md)
- [packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
- [desktop-release-checklist.md](/Users/Shared/chroot/dev/kyuubiki/docs/desktop-release-checklist.md)
