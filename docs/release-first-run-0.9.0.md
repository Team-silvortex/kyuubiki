# First Run Path: v0.9.0

Use this note as the shortest supported path for a first usable `v0.9.0`
session.

This is intentionally opinionated. It does not try to cover every deployment
shape. It gives one path that should feel stable for a first operator session.

## Recommended first-run path

1. Open `Kyuubiki Hub`.
2. Start the local runtime path.
3. Sync the local workload catalog.
4. Open one official sample workload.
5. Launch into Workbench.
6. Run through the orchestrated path first.
7. Review the result/report surface before trying editing-heavy paths.

## Step-by-step

### 1. Open Hub

Use `Kyuubiki Hub` as the first desktop entrypoint for `v0.9.0`.

Why this path:

- it exposes the local runtime state
- it exposes workload sync and sample entrypoints
- it gives the shortest route into Workbench without hand-editing commands

### 2. Start the local path

Inside Hub, prefer the local workstation shape first:

- local frontend
- local orchestrator
- local Rust solver agents

If you are working from the repository directly, the equivalent path is:

- `make hot-local`

Do not start with `cloud` or `distributed` unless that is the specific thing
you are trying to validate.

### 3. Sync the local workload catalog

Inside Hub:

- open `Workload library`
- sync against the local control-plane catalog

This is the supported first-release way to surface official project bundles
without manually finding files first.

### 4. Open one official sample workload

Recommended first samples by family:

- `Axial & Springs`
  `Thermal Bar 1D` or `Spring Chain 1D`
- `Beams & Frames`
  `Cantilever Beam 1D` or `Portal Frame 2D`
- `Trusses`
  `Braced Truss 2D`
- `Planes`
  `Cantilever Plate 2D`

If you only want one sample for a first session, use:

- `Portal Frame 2D`

It exercises modeling, solver routing, result fields, hotspots, and member
force review in one place.

### 5. Launch into Workbench

From Hub:

- use `Open in Workbench`

This is the supported first-run handoff from operator shell to modeling
surface.

### 6. Run through the orchestrated path first

For `v0.9.0`, the recommended first run is:

- Workbench
- orchestrated control-plane path
- then result review

Use direct mesh only when the study coverage table explicitly says it is part
of the first-release path for that study.

Reference:

- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)

### 7. Review the result surface

Before trying deeper editing paths, confirm the release path works by checking:

- the job runs successfully
- the viewport shows the expected result family
- `Inspector > Report` renders a usable summary
- export works for the current study family

That is enough to consider the first-run path healthy.

## First-run success criteria

Treat the first-run path as successful when all of these are true:

- Hub can start or observe the local runtime path
- Hub can sync the local workload catalog
- a supported sample opens in Workbench
- the orchestrated run succeeds
- report/export is usable for that sample

## Not the first-run path

These are valid product paths, but they are not the recommended first step for
`v0.9.0`:

- manually constructing a project bundle before opening Hub
- starting with remote `cloud` or `distributed` orchestration
- validating study families that are marked `minimal` before trying a fully
  supported family
- using direct mesh first when the study coverage table does not list it as the
  supported first-release path

## Related docs

- [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)
- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)
- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [operations.md](/Users/Shared/chroot/dev/kyuubiki/docs/operations.md)
