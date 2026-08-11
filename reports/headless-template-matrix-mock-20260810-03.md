# Headless mock template matrix (manual parse)
- 时间: 2026-08-10 15:46:08 +0800
- 输出目录: /tmp/kyuubiki-template-mock-matrix-20260810
- 说明: 使用原始脚本产物，修正脚本中两处解析问题（`--arg`用法 + validate日志非纯JSON）。

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

## 发现的问题（脚本侧）
1. `jq` 参数组装错误：脚本中用了 `--arg key="$val"` 风格，生成命令为 `--arg key=value`（例如 `--arg validate=0`），导致 `jq` 报 `plan/0 is not defined`。
2. 解析 `validate.out` 时未剥离前置编译日志（`Finished ... Running ...`），`jq` 无法直接读取 JSON，报 `Invalid numeric literal`。

## 对平台行为的结论
- 对这 11 个模板而言，`headless init/validate/plan/render` 与 `headless run --execute --executor mock` 在当次运行中均成功（全部 exit 0，`dry` 和 `execute:mock` 报告 `ok`）。
