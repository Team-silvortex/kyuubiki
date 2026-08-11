# 2026-08-10 服务回归重启 + option6 service 复跑（本轮）

## 目标
- 处理服务重启后端口不可达问题，确认 `127.0.0.1:4000` 可复用。
- 复核上轮 `option6` 的 service 回归（9 个用例）
  - `direct_plane_triangle`
  - `direct_electrostatic_quad`
  - `direct_electrostatic_triangle`
  - `direct_acoustic_bar_1d`
  - `direct_mesh_pipeline`
  - `direct_thermal_truss_3d`
  - `material_structural_panel_screening`
  - `material_dielectric_screening`
  - `material_study_envelope_catalog`

## 重启与连接结论
- `./scripts/kyuubiki start` 在当前沙箱中会触发 `nice(5) failed: operation not permitted`，启动并非稳定。
- 通过权限提升后可启动/维持：
  - 执行：`./scripts/kyuubiki start >/tmp/kyuubiki-start.log 2>&1 & ...`
  - 服务状态与端口监听在一定时段可观察到存在。
- 注意：`./scripts/kyuubiki status` 在某些时刻会显示 stopped，但并非完全同步（可见 `lsof -iTCP:4000 -sTCP:LISTEN` 有 `beam.smp` 监听）。
- Orchestrator 根路径可访问：`curl -sS http://127.0.0.1:4000/` 返回健康 JSON。

## option6 service 执行结果汇总
- 工作目录：`/tmp/kyuubiki-research-option6-service-rerun`
- 统计：`total=9`
- `exec_ok=9`
- `exec_blocked=0`
- `exec_failed=0`
- `dry_ok=8`
- `dry_failed=1`

### 执行异常点
- `opt6_material_study_envelope_catalog` 初次执行在 `dry_run` 被 `--blocked_by_confirmation`（risk=sensitive）阻断。
- 已按你要求补跑 `--execute --allow-sensitive`，结果进入：
  - `exec_status: ok`
  - `exec_steps: 3`
- 当前 `run_exec.out` 已改为可复用结果，`dry` 仍保留敏感阻断语义。

## 直接证据
- [summary.json](/tmp/kyuubiki-research-option6-service-rerun/summary.json)
- [summary.ndjson](/tmp/kyuubiki-research-option6-service-rerun/summary.ndjson)
- [opt6_material_study_envelope_catalog/run_exec.out](/tmp/kyuubiki-research-option6-service-rerun/opt6_material_study_envelope_catalog/run_exec.out)

## 结论（本轮）
- 本轮服务侧 9/9 用例执行通过，未见模板执行失败。
- 仍需把“服务启动路径的沙箱兼容性”（`nice`/状态不一致）作为平台稳定性任务跟进。
