# Headless template matrix
- 时间: 2026-08-12 10:11:19 +0800
- 执行模式: both
- 执行器: mock
- 模板源: custom
- 输出目录: /tmp/kyuubiki-template-matrix-complex

| template | init | validate | plan | render | run_dry | run_exec | dry_status | dry_mode | dry_steps | exec_status | exec_mode | exec_steps | validate_ok | validate_issue_count | exec_validation_ok | exec_validation_issue_count |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct_plane_quad | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_mesh_pipeline | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| material_dielectric_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |
| material_structural_panel_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |
| material_composite_thermo_electric_panel_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |
