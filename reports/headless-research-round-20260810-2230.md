# Headless Research Round - 2026-08-10 15:37 (Mock + Local Port Matrix)

- 时间: 2026-08-10 15:37 CST
- 仓库: `/Users/Shared/chroot/dev/kyuubiki`
- 执行方式: 本地 `kyuubiki` CLI + `--executor mock`（无服务）以及 `headless run` 服务端口回归（本地端口尝试）

## 场景 A：服务端口回归（复测）

目标：复测 `3000/4000` 服务退化+降级行为（使用 1M 网格输入的 `_jobwait_1200000`）

- 输入文件（已就位于可写路径）
  - `/Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_700x700.json`
  - `/Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_1000x1000_noids.json`
  - `/Users/Shared/chroot/dev/kyuubiki/results/headless-all-dryrun-20260804-123750/direct_heat_triangle/input.json`
- 报告输出：`/Users/Shared/chroot/dev/kyuubiki/reports/service-matrix-port-rotation-20260810-2100.md`

结论：
- 三个 case 的 `service_3000` 均为 `failed`。
- 大网格 700/1000 均命中 `frontend_proxy_artifact_limit` 的特征（`service_3000.body-limit-signature=1`），按设计触发 4000 降级。
- `service_4000` 同样失败，错误为：
  - `failed to connect to 127.0.0.1:4000 for model artifact upload within 10000 ms: Operation not permitted (os error 1)`
  - `failed to connect to 127.0.0.1:4000 for service request within 10000 ms: Operation not permitted (os error 1)`（小规模用例）

关键观察：本机环境拒绝本地运行服务端口绑定/连接，导致端到端退化链路无法继续。

## 场景 B：服务器启动可用性（本机）

- `scripts/kyuubiki status`：`orchestrator: stopped`，`agent[5001]: stopped`
- `scripts/kyuubiki start` 报错：`failed to bind 127.0.0.1:5001: Operation not permitted`

结论：本机沙箱环境对关键运行时端口有 OS 权限限制；服务未能启动。

## 场景 C：无服务链路回归（mock 执行）

执行动作（模板）：
1) `headless init --template direct_acoustic_bar_1d`
2) `headless init --template direct_mesh_pipeline`
3) `headless init --template direct_heat_triangle`
4) `headless init --template material_composite_thermo_electric_panel_screening`

结果：
- 四个模板分别 `validate -> ok=true, issues=0`
- `run --executor mock` 全部为 `status=ok`
- 执行步数：3/3/3/9（均符合模板步骤数量）

附：`headless run` 输出文件位于
- `/tmp/kyuubiki-multi-round/direct_acoustic_bar_1d.mock.out`
- `/tmp/kyuubiki-multi-round/direct_mesh_pipeline.mock.out`
- `/tmp/kyuubiki-multi-round/direct_heat_triangle.mock.out`
- `/tmp/kyuubiki-multi-round/material_composite_thermo_electric_panel_screening.mock.out`

## 场景 D：大节点链路压力试验（纯 mock）

1) 合成 30 步工作流（`large_heat_chain_valid`）
- `validate ok=true`，`run --executor mock` `status=ok`，`executed_step_count=30`
- 表明 mock 运行时可吞吐较长链路（>20 步）

2) 合成 30 步“缺失模型字段”无效工作流（`large_heat_chain`）
- `validate ok=false, issue_count=10`（多次 `solve_heat_plane_triangle_2d` 缺少 `model`）
- 但 `run --executor mock` 仍返回 `status=invalid`，`executed_step_count=30`

## 问题清单（当前有效）

- P1：本机沙箱环境下服务端口启动/连接被 `Operation not permitted` 阻断，导致服务端口回退链路无法验证。
- P2：本机无法复现前端/服务迁移链路关键结果，必须有“远端服务可达”或“离线 mock contract 模式”作为默认回归路径。
- P2：`run` 在输入 batch 校验失败时仍可能执行完整 step 列表再给出 `status=invalid`，对真服务而言会产生不必要的资源消耗与误报（见 `large_heat_chain`）。

## 非关键但建议跟进

- `headless init --template ...` 当前会将文件写到仓库根（如 `/Users/Shared/chroot/dev/kyuubiki/material_...headless-workflow.json`），建议增加显式输出路径参数或临时目录，避免长期污染。

## 建议

1) 先修复 `headless run` 的校验短路语义：验证失败应在 `mode=invalid` 时避免“已执行”语义；
2) 在本地 CI/沙箱环境提供可重复的 `orchestrator/frontend` mock service fixture，供服务端口回归脚本不依赖真实端口；
3) 继续保留 `--executor mock` 的大流程验证，作为与真实服务并行的第一层回归。
