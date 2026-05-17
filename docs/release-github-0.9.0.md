# kyuubiki v0.9.0

`v0.9.0` is the release where Kyuubiki starts to feel like a broader
engineering workstation instead of only a browser FEM workbench.

## Highlights

- Wider FEM operator coverage across thermal bars, spring families, beams,
  torsion shafts, frames, and quad plane studies
- Family-aware Workbench study and sample organization
- A more capable `Kyuubiki Hub` for workload launch, runtime watch, and desktop
  readiness
- Repo-level hot-reload flows for local, cloud, and distributed development

## Added and expanded

### Solver family growth

This release broadens first-class operator coverage with:

- `thermal_bar_1d`
- `spring_1d`
- `spring_2d`
- `spring_3d`
- `beam_1d`
- `torsion_1d`
- `frame_2d`
- `plane_quad_2d`

These are wired across:

- Rust solver kernels
- solver RPC
- control-plane job routes
- direct-mesh paths
- Workbench-facing result payloads

### Workbench information architecture

Workbench now groups studies and samples by family:

- `Axial & Springs`
- `Beams & Frames`
- `Trusses`
- `Planes`

That family structure now also flows into `Tree` and `Report`, which makes the
frontend easier to navigate as more solvers land.

### Stronger result analysis surfaces

Examples in this release include:

- plane-field switching for `von Mises / principal stress / max shear`
- hotspot ranking and direct viewport focus for plane studies
- frame member-end force tables with sorting and export
- spring-family force tables and hotspot review
- thermal-bar stress and axial-force review

### Hub becomes more practical

`Kyuubiki Hub` now supports:

- workload-library sync and bundle handling
- desktop readiness and release staging checks
- runtime watch for stack and hot-loop logs
- managed hot-reload control for local, cloud, and distributed loops
- guided assistant entry with audit-aware operator flows

## Why this matters

`v0.9.0` is the point where Kyuubiki becomes structurally easier to grow:

- more studies can land without crowding the UI
- Hub can own more of the desktop operator workflow
- the multi-process development loop is smoother
- release and packaging paths are easier to reason about

## Recommended validation

Before cutting a release, start with:

- `make test-web`
- `make test-rust`
- `make test-frontend`
- `make test-sdk`
- `make test-integration`
- `make test-hub-gui`
- `make test-installer-gui`
- `make test-workbench-gui`
- `make desktop-status PLATFORM=all`

## More detail

- Full notes:
  [release-notes-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-notes-0.9.0.md)
- Short summary:
  [release-summary-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-summary-0.9.0.md)
