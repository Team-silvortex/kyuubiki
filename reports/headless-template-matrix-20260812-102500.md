# Headless template matrix regression

- time: 2026-08-12 10:05:11
- repo: /Users/Shared/chroot/dev/kyuubiki
- workspace: /Users/Shared/chroot/dev/kyuubiki
- api: http://127.0.0.1:3000
- rounds: 1
- headless env: start=900, min=900, max=2500
- template source: discovered
- template count: 33

| template | init | validate | plan | render | dry1 | exec1 | round2_dry | round2_exec | status | err_code |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct_acoustic_bar_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_bar_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_beam_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_electrostatic_quad | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_electrostatic_triangle | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_frame_2d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_frame_3d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_heat_bar_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_heat_quad | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_heat_triangle | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_mesh_pipeline | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_plane_quad | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_plane_triangle | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_spring_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_spring_2d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_spring_3d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_beam_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_frame_2d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_frame_3d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_quad | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_triangle | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_truss_2d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_thermal_truss_3d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_torsion_1d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_truss_2d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| direct_truss_3d | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| material_composite_thermo_electric_panel_screening | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| material_dielectric_screening | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| material_heat_spreader_screening | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| material_structural_panel_screening | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
| material_study_envelope_catalog | 0 | 0 | 0 | 0 | blocked | skipped | n/a | n/a | ok | ok |
| material_study_envelope_ranking | 0 | 0 | 0 | 0 | blocked | skipped | n/a | n/a | ok | ok |
| material_thermo_shield_screening | fail | fail | fail | fail | fail | fail | n/a | n/a | fail | unknown |
