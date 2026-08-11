# Headless mock template matrix (fixed script)
- 时间: 2026-08-10 15:48:48 +0800
- 输出目录: /tmp/kyuubiki-template-mock-matrix-20260810
- 执行器: execute:mock

| template | init | validate | plan | render | run_dry | run_exec | dry_status | dry_mode | dry_steps | exec_status | exec_mode | exec_steps | validate_ok | validate_issue_count | exec_validation_ok | exec_validation_issue_count |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct_acoustic_bar_1d | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_electrostatic_triangle | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_heat_triangle | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_plane_triangle | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_thermal_truss_3d | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_thermal_frame_3d | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| direct_mesh_pipeline | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 3 | ok | execute:mock | 3 | true | 0 | true | 0 |
| material_dielectric_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |
| material_structural_panel_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |
| material_composite_thermo_electric_panel_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |
| material_thermo_shield_screening | 0 | 0 | 0 | 0 | 0 | 0 | ok | dry_run | 9 | ok | execute:mock | 9 | true | 0 | true | 0 |

## 本轮修复
1. 修正 `jq` 参数拼接方式，改为分离 `--arg` 赋值，不再出现 `plan/0 is not defined`。
2. 为 `validate.out` 增加 JSON 抽取（过滤 `Finished/Running` 前置日志），避免 `Invalid numeric literal`。
3. 统一 `run_dry` 与 `run_exec` 状态文件名路径，避免写入/读取不一致导致中断。
