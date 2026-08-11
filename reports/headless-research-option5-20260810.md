# 2026-08-10 选项 5：高价值模板扩展（含新直接算子/流程）

## 目标
- 覆盖未充分触达的直接算子与流程：
  - `direct_plane_triangle`
  - `direct_frame_2d`
  - `direct_beam_1d`
  - `direct_torsion_1d`
  - `direct_thermal_frame_3d`
  - `direct_thermal_beam_1d`
  - `direct_truss_3d`
  - `direct_mesh_pipeline`
  - `browser_capture_review`
- 覆盖更多 workflow/材料方向：
  - `material_study_envelope_catalog`
- 做 2 个 service 回归（用于连通性对比）：
  - `material_structural_panel_screening`
  - `direct_acoustic_bar_1d`

## 跑数与路径
- 工作目录：`/tmp/kyuubiki-research-option5-round`
- 流程：`init -> validate -> plan -> render -> run`（dry） -> `run --execute`（mock 或 service）
- 数据来源：逐用例行目录 + `summary.json`

## 结果（关键）
- `mock`：10 / 10 通过
- `service`：2 / 2 失败

## 成功清单（mock）
- `opt5_direct_plane_triangle`（`direct_plane_triangle`）
- `opt5_direct_frame_2d`（`direct_frame_2d`）
- `opt5_direct_beam_1d`（`direct_beam_1d`）
- `opt5_direct_torsion_1d`（`direct_torsion_1d`）
- `opt5_direct_thermal_frame_3d`（`direct_thermal_frame_3d`）
- `opt5_direct_thermal_beam_1d`（`direct_thermal_beam_1d`）
- `opt5_direct_mesh_pipeline`（`direct_mesh_pipeline`）
- `opt5_browser_capture_review`（`browser_capture_review`）
- `opt5_direct_truss_3d`（`direct_truss_3d`）
- `opt5_material_study_envelope_catalog`（`material_study_envelope_catalog`）

## service 回归
- `opt5_material_structural_panel_screening_service`
  - `dry`: `ok`
  - `execute`: `failed`
  - `exec_mode`: `execute:service`
  - `exec_steps`: `0`
  - `exec_error`: `kyuubiki.headless.transport_failure`
  - 失败点：`solve_plane_quad_2d`（step 1）
  - 连接错误：`failed to connect to 127.0.0.1:4000 for service request within 10000 ms: Operation not permitted (os error 1)`
- `opt5_direct_acoustic_service`
  - `dry`: `ok`
  - `execute`: `failed`
  - `exec_mode`: `execute:service`
  - `exec_steps`: `0`
  - `exec_error`: `kyuubiki.headless.transport_failure`
  - 失败点：`solve_acoustic_bar_1d`（step 1）
  - 连接错误同上

## 直接证据
- [summary.json](/tmp/kyuubiki-research-option5-round/summary.json)
- [summary.ndjson](/tmp/kyuubiki-research-option5-round/summary.ndjson)
- [opt5_material_structural_panel_screening_service/run_exec.out](/tmp/kyuubiki-research-option5-round/opt5_material_structural_panel_screening_service/run_exec.out)
- [opt5_direct_acoustic_service/run_exec.out](/tmp/kyuubiki-research-option5-round/opt5_direct_acoustic_service/run_exec.out)

## 结论
- 本轮为研发场景补上了未覆盖模板（尤其是 `direct_thermal_frame_3d`、`direct_torsion_1d`、`direct_mesh_pipeline`、`browser_capture_review` 与 `material_study_envelope_catalog`）。
- `mock` 侧依旧稳定，说明模板语义与执行链路大方向没问题。
- `service` 侧失败与上轮一致，定位更像服务控制平面/网络连通（`127.0.0.1:4000`）问题。

## 端口替换复测（排查）
- 命令：
  - `headless run <input> --executor service --execute --api-base-url http://127.0.0.1:5108`
- 模板：`direct_plane_triangle`
- 结论：与 4000 一样失败，报错为
  - `failed to connect to 127.0.0.1:5108 ... Operation not permitted (os error 1)`
  - `execution_summary.failure.error_code = kyuubiki.headless.transport_failure`
- 说明：问题在传输连通层，非模板端口配置特例。
