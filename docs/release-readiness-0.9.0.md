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

- [ ] `make test-web`
- [ ] `make test-rust`
- [ ] `make test-frontend`
- [ ] `make test-sdk`
- [ ] `make test-integration`
- [ ] `make test-hub-gui`
- [ ] `make test-installer-gui`
- [ ] `make test-workbench-gui`
- [ ] `make desktop-status PLATFORM=all`

- [ ] release owners agree which failures block release and which only warn

### Failure and support path is minimally usable

- [ ] runtime logs are discoverable
- [ ] Hub/runtime watch is good enough for first-line troubleshooting
- [ ] common failure messages are understandable enough for non-authors
- [x] a short “where to look when things fail” operator note exists

Reference:

- [release-troubleshooting-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-troubleshooting-0.9.0.md)

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
