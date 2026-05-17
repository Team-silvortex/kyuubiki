# Release Summary: v0.9.0

`v0.9.0` is the release where Kyuubiki becomes much more than a browser FEM
workbench. It now reads like a broader engineering workstation with a stronger
desktop operator story, a wider solver family, and a cleaner development loop.

## Headline changes

- broader FEM operator coverage across thermal bars, spring families, beams,
  torsion shafts, frames, and quad plane studies
- a family-aware Workbench that scales better as new study kinds land
- a more capable `Kyuubiki Hub` for workload launch, runtime control, and
  desktop readiness
- repo-level hot-reload entry points for local, cloud, and distributed
  development flows

## What stands out

### Wider solver family

Kyuubiki now supports a much wider set of first-class study kinds, including:

- `thermal_bar_1d`
- `spring_1d`
- `spring_2d`
- `spring_3d`
- `beam_1d`
- `torsion_1d`
- `frame_2d`
- `plane_quad_2d`

These operators are not only added at the kernel level. They now span:

- Rust solver kernels
- solver RPC
- control-plane orchestration
- direct-mesh routing
- Workbench-facing result payloads

### Family-aware Workbench

Workbench no longer presents solver growth as one flat list. `Study` and
`Samples` are now grouped by family:

- `Axial & Springs`
- `Beams & Frames`
- `Trusses`
- `Planes`

That same family language now flows into `Tree` and `Report`, which makes the
UI easier to navigate as the operator surface grows.

### Stronger analysis surfaces

Several study families now have more focused result-review paths, including:

- field switching for plane studies
- hotspot ranking and viewport focus
- frame member-end force tables
- spring-family force tables and exports
- thermal-bar stress and axial-force review

### Hub becomes a practical desktop shell

`Kyuubiki Hub` now behaves much more like a real operator entrypoint:

- workload-library sync and bundle handling
- desktop readiness and release staging checks
- runtime watch for stack and hot-loop logs
- managed hot-reload control for local, cloud, and distributed loops
- guided assistant entry with audit-aware operator flows

## Why this release matters

`v0.9.0` is the point where Kyuubiki starts to feel structurally ready for
continued operator growth:

- more studies can land without crowding the Workbench
- Hub can own more of the desktop operator experience
- the development loop is smoother for multi-process work
- the release path is easier to explain and repeat

## Read more

- full version notes:
  [release-notes-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-notes-0.9.0.md)
- system and runtime shape:
  [system-overview.md](/Users/Shared/chroot/dev/kyuubiki/docs/system-overview.md)
- packaging and release flow:
  [packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
