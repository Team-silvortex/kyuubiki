# Release Study Coverage: v0.9.0

Use this table to review first-release study coverage at the operator level.

This document is narrower than the support matrix. The support matrix answers
“is this family officially in scope?” This table answers “what is the concrete
first-run path for each study?”

## Coverage legend

- `sample`
  an official sample exists in `Library > Samples`
- `orchestrated`
  intended to work through the control plane path
- `direct mesh`
  either supported or explicitly not part of the first release
- `report/export`
  intended to have a usable first-pass result path
- `how to try`
  the shortest supported operator path for the study

## v0.9.0 study coverage

| Study | Status | Official sample | Orchestrated | Direct mesh | Report / export | How to try |
| --- | --- | --- | --- | --- | --- | --- |
| `axial_bar_1d` | fully supported | `Axial Steel Bar` | yes | no | yes | Open `Axial Steel Bar`, run, review axial displacement and stress summary. |
| `thermal_bar_1d` | fully supported | `Thermal Bar 1D` | yes | yes | yes | Open `Thermal Bar 1D`, run, review thermal stress / axial-force report. |
| `spring_1d` | fully supported | `Spring Chain 1D` | yes | yes | yes | Open `Spring Chain 1D`, run, inspect hotspots and force table. |
| `spring_2d` | minimal | `Spring Grid 2D` | yes | yes | yes | Open `Spring Grid 2D`, run, review spring-force distribution and hotspots. |
| `spring_3d` | minimal | `Spring Cage 3D` | yes | yes | yes | Open `Spring Cage 3D`, run, review 3D spring force and displacement output. |
| `beam_1d` | fully supported | `Cantilever Beam 1D` or `Uniform Load Beam 1D` | yes | yes | yes | Open a beam sample, run, switch bending result fields, review end forces. |
| `torsion_1d` | fully supported | `Torsion Shaft 1D` | yes | yes | yes | Open `Torsion Shaft 1D`, run, inspect twist, torque, and shear stress. |
| `frame_2d` | fully supported | `Portal Frame 2D` | yes | yes | yes | Open `Portal Frame 2D`, run, inspect result fields, hotspots, and member forces. |
| `truss_2d` | fully supported | `Braced Truss 2D` | yes | no | yes | Open `Braced Truss 2D`, run, inspect displacement and member force behavior. |
| `truss_3d` | fully supported | `Space Frame Pyramid` | yes | yes | yes | Open `Space Frame Pyramid`, run, inspect 3D member response in immersive view. |
| `plane_triangle_2d` | fully supported | `Cantilever Plate 2D` or `Aluminum Panel` | yes | yes | yes | Open a triangle plane sample, run, switch field view, inspect hotspots. |
| `plane_quad_2d` | fully supported | `Quad Plate Patch 2D` | yes | yes | yes | Open `Quad Plate Patch 2D`, run, switch field view, inspect hotspots. |

## Notes

- `spring_2d` and `spring_3d` are intentionally shipped as `minimal` operator
  families in `v0.9.0`, not as full editing-parity studies.
- `axial_bar_1d` and `truss_2d` are still part of the first-release support
  boundary even though direct-mesh is not part of their initial supported path.
- If the official sample for a study changes, update this table together with:
  - [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
  - [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)

## Related docs

- [release-support-matrix-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-support-matrix-0.9.0.md)
- [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)
- [release-notes-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-notes-0.9.0.md)
