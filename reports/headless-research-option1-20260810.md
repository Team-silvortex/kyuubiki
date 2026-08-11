# 2026-08-10 选项 1：多场景参数化研发验证轮

## 目标
- 继续使用 SDK `headless init/validate/plan/render/run` 验证可扩展研发能力。
- 重点覆盖更偏研发的多物理场模板与编排模板：
  - `direct_electrostatic_quad`
  - `direct_heat_quad`
  - `direct_thermal_frame_3d`
  - `material_dielectric_screening`
  - `workflow_submit_monitor`
  - `material_study_envelope_ranking`
  - `solve_wait_result`

## 执行上下文
- 工作区：`/Users/Shared/chroot/dev/kyuubiki`
- 临时工作流目录：`/tmp/kyuubiki-research-option1-round`
- 命令风格：每例 `validate -> plan -> render -> run(--json)`（dry）及 `run --execute`
- 执行器：`mock` 默认；部分用例补 `service` / `hybrid`；另做一条 `execution-posture=research` 服务连通复测。

## 结果汇总

| case | template | executor | dry | dry-mode | dry steps | exec | exec-mode | exec steps | exec_error |
|---|---|---:|---|---|---:|---|---|---:|---|
| electrostatic_quad_high_perm | direct_electrostatic_quad | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| electrostatic_quad_soft_material | direct_electrostatic_quad | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| heat_quad_radiant | direct_heat_quad | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| heat_quad_low_power | direct_heat_quad | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| thermal_frame_stiff | direct_thermal_frame_3d | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| thermal_frame_expansion | direct_thermal_frame_3d | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| material_dielectric_sweep | material_dielectric_screening | mock | ok | dry_run | 9 | ok | execute:mock | 9 |  |
| workflow_submit_monitor_custom | workflow_submit_monitor | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| workflow_submit_monitor_custom | workflow_submit_monitor | service | ok | dry_run | 3 | failed | execute:service | 0 | kyuubiki.headless.transport_failure |
| material_study_ranking_shift | material_study_envelope_ranking | mock | ok | dry_run | 3 | ok | execute:mock | 3 |  |
| material_study_ranking_shift | material_study_envelope_ranking | hybrid | ok | dry_run | 3 | failed | execute:hybrid | 0 | kyuubiki.headless.transport_failure |
| solve_wait_result_variant | solve_wait_result | mock | ok | dry_run | 4 | ok | execute:mock | 4 |  |
| solve_wait_result_variant | solve_wait_result | service | ok | dry_run | 4 | failed | execute:service | 0 | kyuubiki.headless.transport_failure |

## 关键发现
1. **mock 执行链路稳定**：13 条 mock 跑法中均通过，覆盖 3-9 步长链路，`status=ok`。
2. **服务类执行仍被阻断在连通层**：`service`/`hybrid` 的失败都在 `step 1 service_health`，`error_code=kyuubiki.headless.transport_failure`，`operation not permitted`，`executed_step_count=0`。
3. **execution-posture 为 research 未改变根因**：复测 `--execution-posture research --executor service --api-base-url http://127.0.0.1:4000`，结果同上。
4. **参数化能力生效**：`direct_*` 与 material/template 的输入 JSON 改写（几何/材料/载荷/边界）可在 mock 下稳定通过，说明 SDK 对 workflow payload 的改造入口继续可用。

## 直接证据文件
- `summary.json`：`/tmp/kyuubiki-research-option1-round/summary.json`
- `summary.ndjson`：`/tmp/kyuubiki-research-option1-round/summary.ndjson`
- `execution_posture_service.json`：`/tmp/kyuubiki-research-option1-round/execution_posture_service.json`
- `execution_posture_service.out`：`/tmp/kyuubiki-research-option1-round/execution_posture_service.out`

## 结论
- 这轮没有出现新的 schema/validation 缺陷；`mock` 侧在参数化场景上可继续作为研发回路。
- 真正阻断仍是服务可达性（service/hybrid 及 research posture），集中建议优先做 service 入口可达性治理，降低“连通失败优先级”对业务诊断的遮蔽。
