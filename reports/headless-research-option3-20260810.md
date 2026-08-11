# 2026-08-10 选项 3：模板扩展与新场景回归轮

## 目标
- 覆盖此前较少触达的直接算子模板（机械/热-结构-弹簧族）
  - `direct_plane_quad`
  - `direct_bar_1d`
  - `direct_truss_2d`
  - `direct_frame_3d`
  - `direct_spring_1d/2d/3d`
  - `direct_thermal_quad`
  - `direct_thermal_truss_3d`
- 覆盖未近几轮高价值材料 screening：
  - `material_thermo_shield_screening`
  - `material_heat_spreader_screening`
  - `material_composite_thermo_electric_panel_screening`
- 额外执行一个 service posture 回归命令，确认文档级预检行为。

## 执行方式
- 路径：`/tmp/kyuubiki-research-option3-round`
- 命令序列：`validate -> plan -> render -> run --json`（dry）+ `run --execute`
- 执行器：`mock` 为主，少量 `service` 做连通/预检确认
- 每个输入在 `headless init` 产物基础上进行局部参数化（坐标、刚度/导热/介电、步长偏置、行列名）

## 结果汇总

| case | template | executor | dry | exec | exec_mode | exec_steps | exec_error |
|---|---|---:|---:|---:|---|---:|---|
| direct_plane_quad_stretch | direct_plane_quad | mock | ok | ok | execute:mock | 3 |  |
| direct_bar_1d_stress | direct_bar_1d | mock | ok | ok | execute:mock | 3 |  |
| direct_truss_2d_offset | direct_truss_2d | mock | ok | ok | execute:mock | 3 |  |
| direct_frame_3d_slender | direct_frame_3d | mock | ok | ok | execute:mock | 3 |  |
| direct_spring_2d_soft | direct_spring_2d | mock | ok | ok | execute:mock | 3 |  |
| direct_spring_3d_resilient | direct_spring_3d | mock | ok | ok | execute:mock | 3 |  |
| direct_spring_1d_soft | direct_spring_1d | mock | ok | ok | execute:mock | 3 |  |
| direct_thermal_quad_burn | direct_thermal_quad | mock | ok | ok | execute:mock | 3 |  |
| direct_thermal_truss_3d_gradient | direct_thermal_truss_3d | mock | ok | ok | execute:mock | 3 |  |
| material_thermo_shield_screening_shift | material_thermo_shield_screening | mock | ok | ok | execute:mock | 9 |  |
| material_heat_spreader_screening_shift | material_heat_spreader_screening | mock | ok | ok | execute:mock | 9 |  |
| material_heat_spreader_screening_shift | material_heat_spreader_screening | service | ok | failed | execute:service | 0 | kyuubiki.headless.transport_failure |
| material_composite_thermoelectric_shift | material_composite_thermo_electric_panel_screening | mock | ok | ok | execute:mock | 9 |  |

补充 posture/预检：
- 文件：[material_heat_spreader_service_posture.json](/tmp/kyuubiki-research-option3-round/material_heat_spreader_service_posture.json)
- 结果：`status=invalid`, `error_code=kyuubiki.headless.document_validation`, 消息为 `unsupported headless document schema:`。

## 关键观察
1. **mock 路径健壮性继续良好**：此次新增模板中，全部 mock 用例在 dry 与 execute 均通过。
2. **service 连通与验证分裂**：
   - 已执行的 `service` 路径继续是 `kyuubiki.headless.transport_failure`，说明服务连通问题仍未解除。
   - 这次 posture 命令因为输入文档构造不满足 schema（`document_validation`）而未进入 transport，形成了一个“构造-执行”新样例，说明服务前的文档格式要求很敏感。
3. **可继续扩大量化**：从本轮看 `direct_spring_*`、`direct_frame_3d`、`direct_thermal_truss_3d` 等模板可参数化变体运行稳定。

## 直接证据
- [summary.json](/tmp/kyuubiki-research-option3-round/summary.json)
- [summary.ndjson](/tmp/kyuubiki-research-option3-round/summary.ndjson)

## 结论
- 本轮主要新增：填补 `direct_spring` / `direct_frame` / `direct_truss_2d` 等模板空白的研发验证。
- 未发现新的 mock 运行回归缺陷；优先修复仍是服务层连通与文档构造兼容性两个方向。
