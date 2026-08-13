# Headless research matrix
- 时间: 2026-08-12 13:31:22 +0800
- 执行模式: dry=false mock=false service=true
- 模板源: custom
- 输出目录: /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-complex-stress/run-1
- Service 主/兜底: http://127.0.0.1:3000 / http://127.0.0.1:4000, fallback=1

| template | init | validate | plan | render | validate_ok | validate_issue_count | dry_exit | dry_status | dry_mode | dry_error | mock_exit | mock_status | mock_mode | mock_error | service_primary_exit | service_primary_status | service_primary_mode | service_primary_api | service_primary_error | service_fallback_exit | service_fallback_status | service_fallback_mode | service_fallback_api | service_fallback_error | winner_id | winner_score | winner_field(V/m) | winner_safety |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct_acoustic_bar_1d | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| direct_electrostatic_triangle | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| direct_heat_triangle | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| direct_plane_triangle | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| direct_thermal_frame_3d | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| direct_thermal_truss_3d | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| direct_mesh_pipeline | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| material_dielectric_screening | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | polyimide_film | 0.8 | 30000.0 | 10000.0 |
| material_structural_panel_screening | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | carbon_fiber_quasi_iso | 0.732 | n/a | n/a |
| material_composite_thermo_electric_panel_screening | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | copper_ptfe_glass_epoxy | 0.7300000000000001 | 17500.072625912726 | 3428.55719988023 |
| material_thermo_shield_screening | 0 | 0 | 0 | 0 | true | 0 | -1 | skipped | skipped | n/a | -1 | skipped | skipped | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | invar_36 | 0.75 | n/a | n/a |
