# 2026-08-12 研究轮次：复杂场景复测（mock 与 service 对照）

## 本轮执行环境
- 本地代码仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 执行时间：2026-08-12T02:23:06Z ~ 2026-08-12T02:24:??Z
- 服务端口：未见监听（3000/4000 均无响应）
- 目标：在复杂模板组合下跑通离线 `--executor mock`，并对比 `--executor service` 的可用性边界。

## 1) 多模板 mock 闭环（离线）

### 运行产物
- mock 执行主目录：`/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z`

### 总结
| template | init | validate | plan | render | dry.status | dry.steps | exec.status | winner | mode | material-report schema |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| material_dielectric_screening | 0 | 0 | 0 | 0 | ok | 9 | ok | `n/a`（在 material report 文件里） | execute:mock | kyuubiki.dielectric-material-report/v1 |
| material_structural_panel_screening | 0 | 0 | 0 | 0 | ok | 9 | ok | `n/a`（在 material report 文件里） | execute:mock | kyuubiki.structural-material-report/v1 |
| material_composite_thermo_electric_panel_screening | 0 | 0 | 0 | 0 | ok | 9 | ok | `n/a`（在 material report 文件里） | execute:mock | kyuubiki.composite-panel-report/v1 |
| material_heat_spreader_screening | 0 | 0 | 0 | 0 | ok | 9 | ok | `n/a`（在 material report 文件里） | execute:mock | kyuubiki.material-research-report/v1 |
| direct_electrostatic_triangle | 0 | 0 | 0 | 0 | ok | 3 | ok | `n/a`（无 material report） | execute:mock | n/a |
| direct_thermal_frame_3d | 0 | 0 | 0 | 0 | ok | 3 | ok | `n/a`（无 material report） | execute:mock | n/a |

### 关键观察
- mock 路径在上述 6 个模板均通过 dry-run 与 execute（含 material-report 产出模板）。
- 对于 material 类模板，`round-1-run-exec.json` 当前不携带 `winner_candidate_id`/`report`，需从 material-report 文件中读取 winner（与之前部分脚本假设不一致）。
- `round-1-run-exec.json` 的 schema 为 `kyuubiki.headless-execution-run/v1`，并未包含 `execution_authority` 字段（当前结构里 `HeadlessRunReport` 本身无该字段，行为符合代码结构）。

### 关键产物
- [mock summary](/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z/summary.md)
- [material_dielectric exec](/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z/material_dielectric_screening/headless-loop/round-1-run-exec.json)
- [material_heat_spreader exec](/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z/material_heat_spreader_screening/headless-loop/round-1-material-report.json)
- [direct_thermal_frame_3d exec](/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z/direct_thermal_frame_3d/headless-loop/round-1-run-exec.json)

## 2) 单模板 service smoke（service executor）

### 运行产物
- service smoke 目录：`/Users/Shared/chroot/dev/kyuubiki/runs/service-smoke-20260812T102324Z`

### 关键输出
- Dry-run 成功。
- Execute `--executor service --api-base-url http://127.0.0.1:3000` 失败：
  - run 报文 `status: failed`，`mode: execute:service`。
  - `error_code`: `kyuubiki.headless.transport_failure`
  - `message`: `failed to connect to 127.0.0.1:3000 for service request after 1 bounded attempt(s) with 10000 ms per-address timeout: Operation not permitted (os error 1)`
  - 第 1 步即失败（`solve_electrostatic_plane_quad_2d`）。
- 该失败是 transport 层阻断，与模板/输入本身无关。

### 关键产物
- [service summary](/Users/Shared/chroot/dev/kyuubiki/runs/service-smoke-20260812T102324Z/summary.txt)
- [service exec report](/Users/Shared/chroot/dev/kyuubiki/runs/service-smoke-20260812T102324Z/exec.json)

## 3) heat-spreader chain-next 异构路径复测

### 运行产物
- 目录：`/Users/Shared/chroot/dev/kyuubiki/runs/chain-next-heat-20260812T102350Z`

### 关键结果
- `chain-baseline.json` 与 `chain-replay.json` 均生成成功。
- `winner`: `pyrolytic_graphite_in_plane`
- `round_count`: `2`
- 收敛状态：`blocked_by_quality_gates`（`repair_required=true`，`state` 在 `blocked_by_quality_gates`）
- `CHAIN_ROUNDS=2` 重放可复现。
- 故障注入：
  - `rounds=0` 返回 `--rounds must be at least 1`。
  - 修改 `study` 为 `bogus_study` 返回 `unsupported material study: bogus_study`。

### 关键产物
- [chain baseline](/Users/Shared/chroot/dev/kyuubiki/runs/chain-next-heat-20260812T102350Z/chain-baseline.json)
- [chain replay](/Users/Shared/chroot/dev/kyuubiki/runs/chain-next-heat-20260812T102350Z/chain-replay.json)
- [bad study err](/Users/Shared/chroot/dev/kyuubiki/runs/chain-next-heat-20260812T102350Z/chain-bad-study.err)
- [round0 err](/Users/Shared/chroot/dev/kyuubiki/runs/chain-next-heat-20260812T102350Z/chain-round0.err)

- [plan-next for heat-spreader](/Users/Shared/chroot/dev/kyuubiki/runs/chain-next-heat-20260812T102350Z/heat-plan-next.json)

## 4) 附加验证（mock bad alias）

`--execute --executor mock` 下对 `material_dielectric_screening` 使用 `--material-report study`（不匹配别名）
会在 validation 阶段提前失败：`status=invalid`，`error_code=kyuubiki.headless.material_report_study_unsupported`。

### 关键产物
- [bad alias mock exec](/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z/material_dielectric_screening/headless-loop/bad_alias_mock.err)
- [bad alias mock report](/Users/Shared/chroot/dev/kyuubiki/runs/multi-template-mock-20260812T102310Z/material_dielectric_screening/headless-loop/bad_alias_mock.json)

## 建议
1. 将“复杂模板闭环验证”标准化到固定模板集合（含 material + direct + 复合模板），默认优先 mock 执行作为可复现最小基线，再做 service 通道回归。
2. 在 chain/report 消费侧统一 winner 提取策略：`execute` 结果若无 top-level winner，直接从对应 material-report 文件降级读取，避免假阴性告警。
3. 服务通道异常应在启动阶段做主动预检（health + socket）并给出可自动恢复建议，避免执行中在 step1 即阻断时误判模板回归失败原因。
4. 固定化复现脚本路径与 workspace，减少当前多入口脚本行为差异。

