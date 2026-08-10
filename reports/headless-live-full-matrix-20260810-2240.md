# 2026-08-10 22:40 live executor 全栈矩阵（service / hybrid / mixed 模板）

## 目标
- 按“都搞一下”的要求补齐 live 路线测试：`executor=service`、`executor=hybrid`，并扩展到更多 `direct_*` + `material_*` + `workflow_submit_monitor` 模板。
- 重点确认是否出现 `service/hybrid` 新阻塞模式（除了已知敏感确认）。

## 工作目录与证据
- 模板/结果目录（本轮）：`/private/tmp/kyuubiki_full_round2`、`/private/tmp/kyuubiki_full_round3`
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`

## 1) service/hybrid baseline 复测（已知模板）
| 场景 | 输出 | mode | status | blocked_by_confirmation | execution_failure |
| --- | --- | --- | --- | --- | --- |
| direct_mesh_pipeline（1M） | `service_solve_preview` | execute:service | failed | null | kyuubiki.headless.transport_failure |
| direct_thermal_truss_3d | `service_truss_preview` | execute:service | failed | null | kyuubiki.headless.transport_failure |
| direct_thermal_frame_3d | `service_frame_preview` | execute:service | failed | null | kyuubiki.headless.transport_failure |
| solve_wait_result | `service_solvewait_preview` | execute:service | failed | null | kyuubiki.headless.transport_failure |
| workflow_submit_monitor | `service_wsm_blocked` | execute:service | blocked | {"index":1,"risk":"sensitive"} | null |
| workflow_submit_monitor (+allow-sensitive) | `service_wsm_allow` | execute:service | failed | null | kyuubiki.headless.transport_failure |
| material_study_envelope_ranking | `service_study_blocked` | execute:service | blocked | {"index":1,"risk":"sensitive"} | null |
| material_study_envelope_ranking (+allow-sensitive) | `service_study_allow` | execute:service | failed | null | kyuubiki.headless.transport_failure |
| direct_mesh_pipeline（1M） | `hybrid_solve_preview` | execute:hybrid | failed | null | kyuubiki.headless.transport_failure |
| direct_thermal_frame_3d | `hybrid_frame_preview` | execute:hybrid | failed | null | kyuubiki.headless.transport_failure |
| material_study_envelope_ranking | `hybrid_study_allow` | execute:hybrid | failed | null | kyuubiki.headless.transport_failure |
| material_study_envelope_ranking（默认） | `hybrid_material_study_envelope_ranking_preview` | execute:hybrid | blocked | {"index":1,"risk":"sensitive"} | null |

## 2) 更多混合模板扩展（dry-run + service/hybrid）
| 模板 | dry_run | service | hybrid |
| --- | --- | --- | --- |
| direct_acoustic_bar_1d | 未测（此轮仅拓展 live） | failed | failed |
| direct_electrostatic_triangle | 未测（此轮仅拓展 live） | failed | failed |
| direct_heat_triangle | 未测（此轮仅拓展 live） | failed | failed |
| direct_plane_triangle | 未测（此轮仅拓展 live） | failed | failed |
| material_composite_thermo_electric_panel_screening | ok（dry_run） | failed（allow 与默认均失败） | failed（allow 与默认均失败） |
| material_structural_panel_screening | ok（dry_run） | failed（allow 与默认均失败） | failed（allow 与默认均失败） |
| material_thermo_shield_screening | ok（dry_run） | failed（allow 与默认均失败） | failed（allow 与默认均失败） |
| workflow_submit_monitor | blocked（dry_run） | blocked（默认） | blocked（默认） |
| material_study_envelope_ranking | blocked（dry_run） | blocked（默认） | blocked（默认） |

## 3) 关键错误归因（跨所有 live 路线）
- 统一错误码：`kyuubiki.headless.transport_failure`
- 统一错误消息：`failed to connect to 127.0.0.1:5001 for service request within 10000 ms: Operation not permitted (os error 1)`
- 这类报错出现在 `service`/`hybrid` 的 step1（即第一步提交/提交目录/health 检查）阶段。
- `executor=hybrid` 并未形成“浏览器兜底”路径；在这些场景中仍走服务请求链路。

## 结论
- 目前这两轮没有发现 service/hybrid 的新增参数/策略类 bug；当前阻断特征高度一致，可归为“执行环境不可达”。
- 重复确认：
  - `--allow-sensitive` 仍然只能解除 `sensitive` 阶段的阻断；
  - 在服务层连通失败时仍会返回 `blocked`（当敏感步骤在 step1）或 `failed`（服务未可达）；
  - `dry_run` 在本次模板集中大多数仍可通过（与此前轮次一致）。

## 建议（修复优先级）
1. 优先排查本机/容器内 127.0.0.1:5001 的 service 入口与权限限制（`Operation not permitted`）；
2. 如果同一平台上存在多个 control plane，建议增加 `--api-base-url` 默认容错与快速健康检查提示（避免只报执行失败）；
3. 在 blocked 与 failed 混合场景中，报告可在摘要里显式写明：`service endpoint`, `step_index`, `sensitivity` 三元状态，减少用户试错。

## 核对文件（可直接 grep）
- `/private/tmp/kyuubiki_full_round2/*_preview.*`
- `/private/tmp/kyuubiki_full_round3/*.json`
- `/private/tmp/kyuubiki_full_round3/*.log`
