# Headless Template Matrix Round 4-Lite（2026-08-10）

## 目标
- 再次验证 headless 执行入口在更大模板集合下的执行器适配情况。
- 本轮聚焦 24 个模板，覆盖 `direct_*`、`material_*`、`browser_*` 与 `workflow_*`。
- 每个模板按统一流程执行：`init`、`validate`、`plan`、`render`、`run --dry`、`run --executor {mock|service|hybrid} --allow-sensitive`。

## 数据源
- 结果目录：
  - [/tmp/kyuubiki-template-matrix-round4-lite-20260810](/tmp/kyuubiki-template-matrix-round4-lite-20260810)
- 主汇总：
  - [/tmp/kyuubiki-template-matrix-round4-lite-20260810/summary.json](/tmp/kyuubiki-template-matrix-round4-lite-20260810/summary.json)
- 表格：
  - [/tmp/kyuubiki-template-matrix-round4-lite-20260810/compare-table.md](/tmp/kyuubiki-template-matrix-round4-lite-20260810/compare-table.md)

## 测试模板（24 个）
- `direct_acoustic_bar_1d`
- `direct_plane_quad`
- `direct_plane_triangle`
- `direct_truss_3d`
- `direct_truss_2d`
- `direct_frame_2d`
- `direct_frame_3d`
- `direct_beam_1d`
- `direct_torsion_1d`
- `direct_mesh_pipeline`
- `direct_heat_triangle`
- `direct_heat_bar_1d`
- `direct_thermal_quad`
- `direct_thermal_truss_3d`
- `direct_thermal_frame_3d`
- `direct_thermal_frame_2d`
- `direct_electrostatic_triangle`
- `material_heat_spreader_screening`
- `material_dielectric_screening`
- `material_composite_thermo_electric_panel_screening`
- `material_study_envelope_catalog`
- `workflow_submit_monitor`
- `browser_capture_review`
- `browser_submit_then_poll`

## 聚合结果（最终）
| 执行器 | 样本数 | execute ok | execute failed | transport_failure | executor_compatibility |
|---|---:|---:|---:|---:|---:|
| mock | 24 | 24 | 0 | 0 | 0 |
| service | 24 | 0 | 24 | 22 | 2 |
| hybrid | 24 | 1 | 23 | 23 | 0 |

## 关键发现
1. `mock` 完整通过：24/24。
2. `service` 全部失败，且失败几乎都在执行阶段。
   - `kyuubiki.headless.transport_failure`：22 条
   - `kyuubiki.headless.executor_compatibility`：2 条
3. `hybrid` 仅 1 条通过：`browser_capture_review`。
   - 其余 23 条失败全部是 `transport_failure`。
4. 兼容性失败只出现在 `service` 执行 `browser_*` 模板：
   - `browser_capture_review`
   - `browser_submit_then_poll`

## 证据样例
- service transport 失败样例（`exec-report.json`）
```json
{
  "status": "failed",
  "execution_summary": {
    "failure": {
      "error_code": "kyuubiki.headless.transport_failure",
      "stage": "transport",
      "message": "failed to connect to 127.0.0.1:4000 for service request within 10000 ms: Operation not permitted (os error 1)"
    }
  }
}
```
- service compatibility 失败样例（`browser_capture_review`）
```text
step 1 (open_page) is not compatible with executor service
step 2 (wait) is not compatible with executor service
step 3 (snapshot) is not compatible with executor service
compatible executors: mock, hybrid
```

## 结论
- 当前轮次“真实服务类执行器”问题不是模板本身复杂度问题，核心是控制平面连通性/端口可达性（`127.0.0.1:4000`）导致大规模 `transport_failure`。
- `browser_*` 模板在本地需要 `browser/hybrid` 或明确策略支持 `service`。`service` 并不适配这些步骤。

## 建议（按优先级）
1. 优先修复 `service` 路径的可达性与自检能力：确认 `127.0.0.1:4000` 在当前运行环境中的连通策略。
2. 在执行前置检查中返回更完整的失败上下文（端口、错误码、后端地址），并在报告中持续保留 `error_code/error_message`。
3. 对于 `service` 不能运行的浏览器步骤，给出在工作流选择器中的明确路由提示，避免用户误选。
4. 固定下一轮仅对剩余未覆盖的模板再跑一轮 2-pass 对照（可加内存限额和超时阈值），用于确认无新回归。
