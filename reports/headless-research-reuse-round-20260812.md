# 2026-08-12 Headless research matrix：继续轮次（复用脚本验证）

## 目标
- 验证新脚本 `scripts/run_headless_research_matrix.sh` 的可复用性与稳定性
- 覆盖 dry / mock / service 三路径
- 验证在模块化入口下的可运行性（`labctl`）

## 代码变更
- 新增: [scripts/run_headless_research_matrix.sh](/Users/Shared/chroot/dev/kyuubiki/scripts/run_headless_research_matrix.sh)
- 新增 module: [scripts/modules/headless-research-matrix.sh](/Users/Shared/chroot/dev/kyuubiki/scripts/modules/headless-research-matrix.sh)

## 本轮执行一：dry+mock+service（禁用 fallback）
- 命令: `bash scripts/run_headless_research_matrix.sh --pipeline all --templates direct_electrostatic_triangle,direct_heat_triangle,material_dielectric_screening --workdir .../results/headless-research-matrix-round-20260812 --report-dir .../reports --report-basename headless-research-matrix-round-20260812 --retries 1 --service-fallback 0`
- 报告: [reports/headless-research-matrix-round-20260812-20260812-130908.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round-20260812-20260812-130908.md)
- 总结: 3/3 模板 dry+mock 全部成功；service 因 API 连接失败统一返回 `kyuubiki.headless.transport_failure`。

## 本轮执行二：dry+mock（避免 service 干扰）
- 命令: `bash scripts/run_headless_research_matrix.sh --pipeline dry,mock --templates direct_acoustic_bar_1d,direct_thermal_truss_3d,direct_thermal_frame_3d,direct_mesh_pipeline --workdir .../results/headless-research-matrix-round-20260812-b --report-dir .../reports --report-basename headless-research-matrix-round-20260812-b --retries 1`
- 报告: [reports/headless-research-matrix-round-20260812-b-20260812-130929.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round-20260812-b-20260812-130929.md)
- 总结: 4/4 模板 dry+mock 全部成功；service 被跳过。

## 发现问题与修复
- 问题: 首次运行时报 `material_args[@]: unbound variable`（`set -u` 下数组未安全展开）。
- 复现模板: 直接在 `service` 路径执行时。
- 修复: `run_headless_research_matrix.sh` 中将 `material_args` 明确声明为数组 (`local -a`) 并使用 `${material_args[@]+...}` 安全展开。
- 修复状态: 已修复。

## 下一步建议
1. 启动 service 后台（3000）重跑执行一次 `pipeline=service`，验证真实端到端。
2. 如 service 报 3000/4000 体积/传输限流，再跑一次 `--service-fallback 1` 验证 4000 自动兜底链路。
3. 继续把 `--templates` 加大到 `TEMPLATES_DIRECT + TEMPLATES_MATERIAL` 全量。
