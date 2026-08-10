# 2026-08-10 22:10 长链路 + 大规模网格 + 编排复测（第3轮）

## 目标
- 复测一套“偏真实研发场景”的 headless 工作流：大规模网格（1M 元素）、热/力直接算子编排链、以及可提交/监控/取结果链路。
- 在不引入额外脚本缺陷的前提下，区分“真正可复现的问题”和“流程预期行为”。

## 证据
- 工作目录：`/private/tmp/kyuubiki_longhaul_round`
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 变更模板：
  - `direct_mesh_pipeline_1m.workflow.json`（来自 `direct_mesh_pipeline`，将 `elements` 强制为 `1000000`）
  - `orchestration_chain_small.workflow.json`（基于 `direct_thermal_truss_3d`）
  - `orchestration_chain_medium.workflow.json`（基于 `direct_thermal_frame_3d`）
  - `workflow_submit_monitor.workflow.json`
  - `solve_wait_result.workflow.json`
  - `material_study_envelope_ranking.workflow.json`

## 回归矩阵（本轮）
| 场景 | 运行输出 | mode | status | executed_step_count | blocked_by_confirmation |
| --- | --- | --- | --- | --- | --- |
| direct_thermal_frame_3d dry_run | chain_medium_dry | dry_run | ok | 3 | null |
| direct_thermal_truss_3d dry_run | chain_small_dry | dry_run | ok | 3 | null |
| direct_mesh_pipeline 1M dry_run | mesh_1m_dry | dry_run | ok | 3 | null |
| solve_wait_result dry_run | solve_wait_dry | dry_run | ok | 4 | null |
| workflow_submit_monitor dry_run | wsm_dry | dry_run | blocked | 2 | {"index":1,"risk":"sensitive"} |
| direct_thermal_frame_3d execute:mock | chain_medium_exec_mock | execute:mock | ok | 3 | null |
| direct_thermal_truss_3d execute:mock | chain_small_exec_mock | execute:mock | ok | 3 | null |
| direct_mesh_pipeline 1M execute:mock | mesh_1m_exec_mock | execute:mock | ok | 3 | null |
| solve_wait_result execute:mock | solve_wait_exec_mock | execute:mock | ok | 4 | null |
| workflow_submit_monitor execute:mock | wsm_exec_mock | execute:mock | blocked | 0 | {"index":1,"risk":"sensitive"} |
| material_study_envelope_ranking 默认敏感阻断 | material_study_blocked | execute:mock | blocked | 0 | {"index":1,"risk":"sensitive"} |
| material_study_envelope_ranking allow-sensitive | material_study_allow_preview | execute:mock | ok | 3 | null |

## 发现与结论

### 1) 已复测通过（可用）
- 1M 元素参数确已进入 `direct_mesh_solve` 负载，`elements` 字段为 `1000000`（`element_count` 为 `null`），且 dry-run 与 execute:mock 均为 `status=ok`。
- 直接解耦的热力/力学编排链（`truss_3d`、`frame_3d`）在 dry-run 与 mock 下全部 `ok`，执行步数符合预期（3 步）。
- `solve_wait_result` 在 dry-run 与 mock 下行为稳定，`service_health -> create -> wait -> fetch` 全通过（mock 预期结果为模拟结果）。

### 2) 未解耦的确认行为（仍需提示位）
- `workflow_submit_monitor` 与 `material_study_envelope_ranking` 的 step1 均因 `sensitive` 阻断（`index:1`）：
  - 这是“默认阻断、可通过 `--allow-sensitive` 放行”的一致行为，已在此前轮次也确认。
  - 这类阻断属于策略安全边界，不是脚本执行错误。

### 3) 无新增阻塞性 Bug
- 在本轮执行矩阵中未复现新的参数解析/执行器错误（除前述策略敏感确认外）。
- 上次定位到的 `runtime_style` 一类问题本轮未被这组用例覆盖；若要复核，请继续独立执行 `runtime-style` 回归。

## 修复建议（后续可选）
1. 在 `dry_run` 阶段输出更清晰的敏感阻断文案（建议同时输出：
   - 风险类型
   - 阻断 step index
   - 可用开关（如 `--allow-sensitive`）和适用前提）。
2. `workflow_submit_monitor` 的 step1 为敏感路径时，可在报告摘要里增加一行 `remediation_hint`，减少用户试错成本。

## 产出文件清单（便于核验）
- `/private/tmp/kyuubiki_longhaul_round/direct_mesh_pipeline_1m.workflow.json`
- `/private/tmp/kyuubiki_longhaul_round/chain_small_dry.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/chain_small_exec_mock.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/chain_medium_dry.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/chain_medium_exec_mock.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/mesh_1m_dry.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/mesh_1m_exec_mock.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/wsm_dry.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/wsm_exec_mock.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/solve_wait_dry.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/solve_wait_exec_mock.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/material_study_blocked.json/.log`
- `/private/tmp/kyuubiki_longhaul_round/material_study_allow_preview.json/.log`
