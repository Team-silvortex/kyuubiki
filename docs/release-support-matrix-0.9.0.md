# Release Support Matrix: v0.9.0

This document defines the official study-family support boundary for the
initial usable `v0.9.0` release.

Use it to answer:

- which studies are officially first-release targets
- which ones are `fully supported` versus `minimal/experimental`
- what level of Workbench/operator path exists for each study

## Support levels

- `fully supported`
  sample, Workbench entry, orchestrated run path, result/report/export path,
  and a reasonably complete first-pass analysis surface are present
- `minimal`
  the study is usable and intentionally shipped, but editing or analysis parity
  is still lighter than the stronger first-release families
- `later`
  not part of the first usable release target

## v0.9.0 matrix

| Study | Family | Status | Sample | Orchestrated | Direct mesh | Report / export | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `axial_bar_1d` | Axial & Springs | fully supported | yes | yes | no | yes | baseline 1D axial study |
| `thermal_bar_1d` | Axial & Springs | fully supported | yes | yes | yes | yes | temperature-driven axial stress path is present |
| `spring_1d` | Axial & Springs | fully supported | yes | yes | yes | yes | hotspot and force-table path present |
| `spring_2d` | Axial & Springs | minimal | yes | yes | yes | yes | lighter editing parity than stronger line-study families |
| `spring_3d` | Axial & Springs | minimal | yes | yes | yes | yes | usable analysis path, lighter modeling surface |
| `beam_1d` | Beams & Frames | fully supported | yes | yes | yes | yes | distributed load, field switching, end-force review |
| `torsion_1d` | Beams & Frames | fully supported | yes | yes | yes | yes | twist / torque / stress review path present |
| `frame_2d` | Beams & Frames | fully supported | yes | yes | yes | yes | dedicated hotspots, member-force table, result-aware tree |
| `truss_2d` | Trusses | fully supported | yes | yes | no | yes | strongest direct editing path in 2D line families |
| `truss_3d` | Trusses | fully supported | yes | yes | yes | yes | immersive 3D workflow and spatial editing path |
| `plane_triangle_2d` | Planes | fully supported | yes | yes | yes | yes | field switching and hotspot ranking present |
| `plane_quad_2d` | Planes | fully supported | yes | yes | yes | yes | solver and Workbench path are present |

## Release rule

For `v0.9.0`, the official first usable release boundary is:

- all `fully supported` studies above
- `spring_2d` and `spring_3d` are shipped as `minimal` operator families, not
  as parity-complete editing surfaces

If any study changes level, update this matrix before updating public release
copy.

## Related docs

- [release-readiness-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-readiness-0.9.0.md)
- [release-github-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-github-0.9.0.md)
- [release-summary-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-summary-0.9.0.md)
- [release-notes-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-notes-0.9.0.md)
