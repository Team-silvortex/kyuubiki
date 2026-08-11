# Headless Template Matrix Round 5（2026-08-10）

## 目标
对上轮剩余模板做补齐验证（`mock` / `service` / `hybrid`），确认真实执行链条在不同模板族的行为一致性。

## 覆盖模板（13）
- solve_wait_result
- material_thermo_shield_screening
- material_structural_panel_screening
- material_study_envelope_ranking
- direct_bar_1d
- direct_spring_2d
- direct_spring_3d
- direct_spring_1d
- direct_heat_quad
- direct_thermal_triangle
- direct_thermal_truss_2d
- direct_thermal_beam_1d
- direct_electrostatic_quad

## 数据
- 结果目录：
  - [/tmp/kyuubiki-template-matrix-round5-20260810](/tmp/kyuubiki-template-matrix-round5-20260810)
- 汇总：
  - [/tmp/kyuubiki-template-matrix-round5-20260810/summary.json](/tmp/kyuubiki-template-matrix-round5-20260810/summary.json)
  - [/tmp/kyuubiki-template-matrix-round5-20260810/compare-table.md](/tmp/kyuubiki-template-matrix-round5-20260810/compare-table.md)
  - [/tmp/kyuubiki-template-matrix-round5-20260810/aggregate.json](/tmp/kyuubiki-template-matrix-round5-20260810/aggregate.json)

## 聚合结果
- 总记录数：39（13 模板 × 3 执行器）
- mock：13/13 通过
- service：0/13 通过
- hybrid：0/13 通过

### 执行失败归因
- `service`：`kyuubiki.headless.transport_failure` ×13
- `hybrid`：`kyuubiki.headless.transport_failure` ×13
- `executor_compatibility`：0

## 证据样例
- `service` 执行失败（`solve_wait_result`）
```json
{
  "status": "failed",
  "execution_summary": {
    "failure": {
      "error_code": "kyuubiki.headless.transport_failure",
      "message": "failed to connect to 127.0.0.1:4000 for service request within 10000 ms: Operation not permitted (os error 1)"
    }
  }
}
```
- `hybrid` 执行失败同样是 service 入口连通问题，`required_engines` 标记为 `service`。

## 结论
- 这 13 个模板在 `mock` 下全部可通过，说明模板构建链路（init/validate/plan/render/run dry）健康。
- `service` 与 `hybrid` 的失败是纯执行层连通问题（统一连到 `127.0.0.1:4000`，统一报 transport），没有发现新的兼容性错误。
- 当前证据链支持修复优先级：先恢复 service runtime 网络可达性，再谈模板级兼容性回归。

## 与上一轮（Round 4-Lite）对齐
- Round 4-Lite 已经显示大量 `service` 的连通失败与 `hybrid` 偶发通过。
- Round 5 结果保持一致，表明问题是环境/服务端健康，不是模板个体。
