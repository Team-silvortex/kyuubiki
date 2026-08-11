# Kyuubiki 研究课题扩展轮次报告（Round 3，2026-08-10）

## 目标
进一步扩大研究课题并做执行器对照：
- 评估 mock 与 service 在高复杂模板下的一致性
- 验证带 `--allow-sensitive` 的真实执行行为
- 用 hybrid 验证 browser+service 混合模板是否可运行

## 测试集
- 模板数：11
- 选择模板：
  - `direct_plane_quad`
  - `direct_thermal_frame_3d`
  - `direct_mesh_pipeline`
  - `material_heat_spreader_screening`
  - `material_composite_thermo_electric_panel_screening`
  - `material_study_envelope_catalog`
  - `workflow_submit_monitor`
  - `browser_submit_then_poll`
  - `direct_truss_3d`
  - `direct_thermal_truss_3d`
  - `direct_heat_triangle`
  - `browser_capture_review`

## 结果来源
- 逐模板执行记录（含 dry + execute + 报告 JSON）：
  - [/tmp/kyuubiki-template-matrix-compare-20260810](/tmp/kyuubiki-template-matrix-compare-20260810)
- 汇总文件：
  - [/tmp/kyuubiki-template-matrix-compare-20260810/compare.json](/tmp/kyuubiki-template-matrix-compare-20260810/compare.json)
- 汇总表：
  - [/tmp/kyuubiki-template-matrix-compare-20260810/compare-table.txt](/tmp/kyuubiki-template-matrix-compare-20260810/compare-table.txt)
- hybrid 结果：
  - [/tmp/kyuubiki-template-matrix-compare-20260810/hybrid.ndjson](/tmp/kyuubiki-template-matrix-compare-20260810/hybrid.ndjson)

## 关键结论
### 1) mock 执行器
- `dry: ok`，`execute: ok`：11/11
- 无阻塞（`dry` 和 `execute` 全部通过）

### 2) service 执行器
- `dry: ok`：11/11
- `execute: ok`：0/11
- `execute` 失败分类：
  - `kyuubiki.headless.transport_failure`（10 个）：连接 `127.0.0.1:4000` 被拒（`Operation not permitted`）
  - `kyuubiki.headless.executor_compatibility`（1 个）：`browser_submit_then_poll` 的 `open_page`/`click` 与 `service` 不兼容

### 3) hybrid 执行器（抽样 4）
- `browser_capture_review`：`dry ok` / `execute ok`
- `browser_submit_then_poll`：`dry ok`，`execute failed`，原因同样是 `transport_failure`
- `workflow_submit_monitor`、`direct_plane_quad`：`execute failed` 为 `transport_failure`

### 4) 风险/策略层信号（与模板语义无关）
- 在不加 `--allow-sensitive` 的跑法里，`material_study_envelope_*`、`workflow_submit_monitor`、部分 browser 模板会被 policy 阻断。
- `browser_*` 合成模板 `needs_desktop_browser=true`，需桌面浏览器能力前置。

### 5) 可复现问题
- `service` 与 `hybrid` 的非 browser-only 步骤在当前环境都会卡在与 `127.0.0.1:4000` 的 transport 连接，属于执行端可达性问题（高概率是环境未起服务/网络隔离），不是模板本体。
- `browser_submit_then_poll` 在 `service` 下会被兼容性检查挡掉（设计上应使用 `hybrid`）。

## 建议
1. 先排查服务后端连通性：确保 `127.0.0.1:4000` 服务可达（或改为可达 endpoint）。
2. 在 CI/研发里对 `--allow-sensitive` 与 `needs_desktop_browser` 做显式允许策略，避免人为误挡。
3. 保留 `hybrid` 作为 browser+service 混合模板的默认执行器。
4. 清洗测试 harness：当前本轮 `validate.out` 输出带 rust 命令前置信息，导致 `issue_count` 解析脚本里被误判；建议提取 JSON 段后再 parse。
