# 2026-08-10 选项 4：实战方向扩展（电磁/声学/流程）

## 目标
- 使用同一套 `validate -> plan -> render -> run` 链路扩展到未触达的高价值模板：
  - 电磁直接求解：`direct_electrostatic_quad`、`direct_electrostatic_triangle`
  - 声学直接求解：`direct_acoustic_bar_1d`
  - 热类直接求解：`direct_heat_triangle`、`direct_heat_bar_1d`
  - 热-力耦合：`direct_thermal_triangle`、`direct_thermal_truss_2d`、`direct_thermal_frame_2d`
  - 材料筛选：`material_dielectric_screening`、`material_structural_panel_screening`
  - 材料流程：`material_study_envelope_ranking`、`workflow_submit_monitor`
- 单独补一条 service 回归：`material_dielectric_screening`（service）

## 运行环境
- 工作目录：`/tmp/kyuubiki-research-option4-round`
- 真实用例文件：目录下 12 个 mock 用例 + 1 个 service 用例
- 执行命令：`validate`、`plan`、`render`、`run --json`、`run --json --execute --executor <mock|service>`

## 结果汇总（关键）
- `mock` 用例数：12 / 12 全部通过
- `service` 用例数：1 / 1 全部失败于连接层，命中 `kyuubiki.headless.transport_failure`

### pass 清单（mock）
- `direct_electrostatic_quad` / `direct_electrostatic_triangle` / `direct_acoustic_bar_1d`
- `direct_heat_triangle` / `direct_heat_bar_1d`
- `direct_thermal_triangle` / `direct_thermal_truss_2d` / `direct_thermal_frame_2d`
- `material_dielectric_screening` / `material_structural_panel_screening`
- `material_study_envelope_ranking` / `workflow_submit_monitor`

### service 失败清单
- `material_dielectric_screening_service_only`
  - `dry`: `ok`
  - `execute`: `failed`
  - `exec_mode`: `execute:service`
  - `exec_error_code`: `kyuubiki.headless.transport_failure`
  - 现象：连接 `127.0.0.1:4000` 被拒（operation not permitted），未进入业务求解阶段

### 直接证据
- [summary.json](/tmp/kyuubiki-research-option4-round/summary.json)
- [summary.ndjson](/tmp/kyuubiki-research-option4-round/summary.ndjson)
- `material_dielectric_screening_service_only/run_exec.out`
- `material_dielectric_screening_service_only/run_exec.err`

## 观察
1. `mock` 模式在电磁、声学、热传导、热-力、材料筛选、workflow 这些新场景均稳定通过，说明 `mock` 的 pipeline（validate/plan/render/execute）在这批真实型模板中表现稳定。
2. `service` 失败不再是单模板异常，表现与上轮一致：
   - 同样是 transport 层失败，错误码 `kyuubiki.headless.transport_failure`
   - 与模板功能本身无关（dry 都成功）
3. 与上轮衔接：本轮把 service 失败定位再次确认到连接层，建议下一轮集中修复服务端口/网络/控制面后再做 `service` 回归。

## 结论
- 本轮目标场景（更贴近真实研发链路）已覆盖，并无新的 `mock` 回归缺陷。
- 持续阻塞点仍是服务层连通：`service` 执行稳定卡在 transport，建议把该项作为版本里程碑前置问题优先处理。
