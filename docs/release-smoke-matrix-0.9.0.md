# Release Smoke Matrix: v0.9.0

Use this matrix to track the minimum first-release study validation pass.

This document is intentionally operational. It is not a support statement. It
exists to answer a narrower question:

- did we actually walk the official first-run path for each release study?

## How to use this matrix

For each study, validate the official path in this order:

1. open the official sample
2. confirm the sample opens in Workbench
3. confirm the orchestrated run completes
4. confirm the result/report surface is usable
5. confirm export is usable

Direct mesh is included only when it is part of the documented first-release
path for that study.

References:

- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)
- [release-first-run-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-first-run-0.9.0.md)

## Matrix

| Study | Sample opens in Workbench | Orchestrated run | Direct mesh | Report usable | Export usable | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `axial_bar_1d` | [x] | [x] | n/a | [x] | [x] | Official sample: `Axial Steel Bar`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_displacement≈8.57e-7`, `max_stress≈150000`, `elements=6`. |
| `thermal_bar_1d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Thermal Bar 1D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_stress=100800000`, `max_axial_force=1008000`. |
| `spring_1d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Spring Chain 1D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_displacement=0.001`, `max_force=1000`, `elements=2`. |
| `spring_2d` | [ ] | [ ] | [ ] | [ ] | [ ] | `minimal` family in `v0.9.0` |
| `spring_3d` | [ ] | [ ] | [ ] | [ ] | [ ] | `minimal` family in `v0.9.0` |
| `beam_1d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Cantilever Beam 1D` or `Uniform Load Beam 1D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_moment≈2000`, `max_stress≈1.25e7`. |
| `torsion_1d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Torsion Shaft 1D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_torque=1200`, `max_stress=6000000`, `max_rotation≈0.0015`. |
| `frame_2d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Portal Frame 2D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_moment≈2000`, `max_stress≈1.25e7`. |
| `truss_2d` | [x] | [x] | n/a | [x] | [x] | Official sample: `Braced Truss 2D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_displacement≈1.10e-6`, `max_stress≈58962.38`, `elements=3`. |
| `truss_3d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Space Frame Pyramid`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17` with the official sample; `max_displacement≈1.58e-6`, `nodes=4`, `elements=6`. A simplified one-member payload was singular and is not the release path. |
| `plane_triangle_2d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Cantilever Plate 2D` or `Aluminum Panel`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_stress≈77475.40`, `max_displacement≈1.12e-6`, `elements=2`. |
| `plane_quad_2d` | [x] | [x] | [ ] | [x] | [x] | Official sample: `Quad Plate Patch 2D`. Sample open in Workbench verified on `2026-05-17`. Report/export path in Workbench verified on `2026-05-17`. Local orchestrated smoke passed on `2026-05-17`; `max_stress≈60221.34`, `elements=1`. |

## Exit expectation

Treat a study row as release-ready when:

- every non-`n/a` column is checked
- any deviations are written down in `Notes`
- the result matches the support level claimed in the support matrix

## Related docs

- [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)
- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [release-study-coverage-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-study-coverage-0.9.0.md)
- [release-first-run-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-first-run-0.9.0.md)
