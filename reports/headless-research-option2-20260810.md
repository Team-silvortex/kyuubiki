# 2026-08-10 选项 2：模板扩展/混合路径回归轮

## 目标
- 继续做“真实研发问题”级别覆盖，补齐本地未大规模跑过的模板族：
  - `direct_truss_3d`
  - `direct_frame_2d`
  - `direct_beam_1d`
  - `direct_torsion_1d`
  - `direct_thermal_truss_2d`
  - `material_heat_spreader_screening`
  - `material_structural_panel_screening`
  - `material_study_envelope_catalog`
  - `browser_capture_review`
  - `browser_submit_then_poll`
- 验证流程：`validate -> plan -> render -> run(dry)` + `run --execute`。
- 兼容性补测：选取一条链路做 `execution-posture research + service`。

## 执行环境
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 临时工作区：`/tmp/kyuubiki-research-option2-round`

## 本轮结果（摘要）

| case | template | executor | dry | exec | exec_mode | exec_steps | exec_error |
|---|---|---:|---:|---:|---|---:|---|
| direct_truss_3d_scale_up | direct_truss_3d | mock | ok | ok | execute:mock | 3 |  |
| direct_frame_2d_stiff | direct_frame_2d | mock | ok | ok | execute:mock | 3 |  |
| direct_beam_1d_thin | direct_beam_1d | mock | ok | ok | execute:mock | 3 |  |
| direct_torsion_1d_shift | direct_torsion_1d | mock | ok | ok | execute:mock | 3 |  |
| direct_thermal_truss_2d_drift | direct_thermal_truss_2d | mock | ok | ok | execute:mock | 3 |  |
| material_heat_spreader_screening_shift | material_heat_spreader_screening | mock | ok | ok | execute:mock | 9 |  |
| material_structural_panel_screening_shift | material_structural_panel_screening | mock | ok | ok | execute:mock | 9 |  |
| material_structural_panel_screening_shift | material_structural_panel_screening | service | ok | failed | execute:service | 0 | kyuubiki.headless.transport_failure |
| material_study_envelope_catalog_shift | material_study_envelope_catalog | mock | ok | ok | execute:mock | 3 |  |
| material_study_envelope_catalog_shift | material_study_envelope_catalog | hybrid | ok | failed | execute:hybrid | 0 | kyuubiki.headless.transport_failure |
| browser_capture_review | browser_capture_review | mock | ok | ok | execute:mock | 3 |  |
| browser_submit_then_poll | browser_submit_then_poll | mock | ok | ok | execute:mock | 5 |  |
| browser_submit_then_poll | browser_submit_then_poll | hybrid | ok | failed | execute:hybrid | 2 | kyuubiki.headless.transport_failure |

- `summary`: [summary.json](/tmp/kyuubiki-research-option2-round/summary.json)
- `summary.ndjson`: [summary.ndjson](/tmp/kyuubiki-research-option2-round/summary.ndjson)

## 关键发现
1. **mock 通路稳态再次确认**：10/13?（含 service/hybrid 的失败）中，mock 样例均通过，执行步数与模板步长一致（3/5/9）。
2. **`browser` 模板行为**：`browser_capture_review` 与 `browser_submit_then_poll` 的 `mock` 执行在本地正常通过（分别 3 步、5 步），说明浏览器模板在工作流执行层面可被 mock 化验证。
3. **服务/混合仍是连通问题主导**：`service` 与 `hybrid` 路径一律在 `service_health`/第一步前置附近触发 `kyuubiki.headless.transport_failure`（`executed_step_count=0`）。
4. **execution-posture 并未改变根因**：对 `material_study_envelope_catalog_shift` 的 `--execution-posture research --executor service --api-base-url http://127.0.0.1:4000` 同样返回 transport_failure。

## 下一步建议
- 继续优先修复服务链路可达性（127.0.0.1:4000）+ 连接前置的错误分层。
- 在服务可达前，可继续用 mock 拉满模板覆盖（高覆盖率回归），避免“连通失败”掩盖真实验证失败。
