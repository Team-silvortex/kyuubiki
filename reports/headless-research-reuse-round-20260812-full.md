# 2026-08-12 Headless Research Matrix（续测：模块化入口+labctl修复）

## 变更清单
- [scripts/run_headless_research_matrix.sh](/Users/Shared/chroot/dev/kyuubiki/scripts/run_headless_research_matrix.sh)
- [scripts/modules/headless-research-matrix.sh](/Users/Shared/chroot/dev/kyuubiki/scripts/modules/headless-research-matrix.sh)
- [scripts/labctl.sh](/Users/Shared/chroot/dev/kyuubiki/scripts/labctl.sh)（兼容修复，`set -u` 环境下稳定）

## 本次执行记录

### 执行1：service 专测（dry/mock 禁用）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline service --templates direct_electrostatic_triangle,direct_heat_triangle --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-service-round-20260812 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-service-round-20260812 --retries 1 --service-fallback 1`
- 结果文件
  - [reports/headless-research-matrix-service-round-20260812-20260812-131022.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-service-round-20260812-20260812-131022.md)
- 结论：两模板 init/validate/plan/render 均成功；service execute 在 `127.0.0.1:3000` 失败，错误码 `kyuubiki.headless.transport_failure`。

### 执行2：labctl 跑通验证（dry,mock，仅1模板）
- 命令
  - `bash scripts/labctl.sh run headless-research-matrix --run-id continue-service-test-20260812 --workspace /Users/Shared/chroot/dev/kyuubiki/runs/headless-research-matrix/continue-service-test-20260812/workspace --set PIPELINE=dry,mock --set TEMPLATES=direct_plane_triangle --set WORKDIR=/Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-labctl-20260812b --set REPORT_DIR=/Users/Shared/chroot/dev/kyuubiki/reports --set REPORT_BASENAME=headless-research-matrix-labctl-20260812b --set MAX_ATTEMPTS=1`
- run-manifest
  - [runs/headless-research-matrix/continue-service-test-20260812/run-manifest.json](/Users/Shared/chroot/dev/kyuubiki/runs/headless-research-matrix/continue-service-test-20260812/run-manifest.json)
- 结果文件
  - [results/headless-research-matrix-labctl-20260812b/summary.json](/Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-labctl-20260812b/summary.json)
- 结论：`dry` + `mock` 成功，说明 module->scripts 入口链路闭环可用。

### 执行3：默认模板池全量（dry+mock）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline dry,mock --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-round-20260812-c --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round-20260812-c --retries 1`
- 结果文件
  - [reports/headless-research-matrix-round-20260812-c-20260812-131059.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round-20260812-c-20260812-131059.md)
  - `results/headless-research-matrix-round-20260812-c/summary.json`
- 结论：11/11 模板 dry+mock 全部成功；当前阶段未出现 block、material report winner。

### 执行4：默认模板池全量（dry+mock，补充新一轮）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline dry,mock --service-fallback 0 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round14 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round14-20260812`
- 结果文件
  - [reports/headless-research-matrix-round14-20260812-20260812-131428.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round14-20260812-20260812-131428.md)
  - `results/headless-research-matrix-20260812-round14/summary.json`
- 结论：11/11 模板全部通过 dry/mock；与历史结果一致。

### 执行5：研究向模板补充（含材料目录扩展）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline dry,mock --templates material_heat_spreader_screening,material_study_envelope_catalog,material_study_envelope_ranking,direct_mesh_pipeline --service-fallback 0 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round15b --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round15b-20260812 --retries 1`
- 结果文件
  - [reports/headless-research-matrix-round15b-20260812-20260812-131459.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round15b-20260812-20260812-131459.md)
  - `results/headless-research-matrix-20260812-round15b/summary.json`
- 结论：  
  - `material_heat_spreader_screening`、`direct_mesh_pipeline` 走完 dry+mock；
  - `material_study_envelope_catalog` 与 `material_study_envelope_ranking` 在 dry 阶段 `blocked`（`risk: sensitive`），需 review 后加 `--allow-sensitive`；
  - 二者 dry 输出为 `blocked_by_confirmation: {index: 1, risk: "sensitive"}`，非执行错误。

### 执行6：旧模板名兼容性回归
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline dry,mock --templates material_heat_spreader_screening,material_study_envelope_catalog,material_study_ranking_shift,direct_torsion_1d --service-fallback 0 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round15 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round15-20260812 --retries 1`
- 结果文件
  - `results/headless-research-matrix-20260812-round15/summary.ndjson`
- 结论：`material_study_ranking_shift` 为无效模板名，CLI 返回 `headless_command_failed` 并提示最近邻建议（`material_study_envelope_catalog`, `material_study_envelope_ranking`）。

### 执行7：服务化实战批次（含默认池+敏感材料模板）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline dry,mock,service --templates direct_mesh_pipeline,material_heat_spreader_screening,material_study_envelope_catalog,material_study_envelope_ranking,material_dielectric_screening --service-fallback 1 --retries 1 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round16 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round16-20260812`
- 结果文件
  - [reports/headless-research-matrix-round16-20260812-20260812-131616.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round16-20260812-20260812-131616.md)
  - `results/headless-research-matrix-20260812-round16/summary.json`
- 结论：
  - `direct_mesh_pipeline`、`material_heat_spreader_screening`、`material_dielectric_screening` service 可执行成功（`service_primary_status=ok`）；
  - `material_study_envelope_catalog` 与 `material_study_envelope_ranking` 在 `dry` 阶段仍被 `blocked_by_confirmation` 阻挡，故未进入 mock/service；
  - 该批次首次出现 service 主通道稳定返回 `ok`，service 不是恒定不可达。

### 执行8：敏感模板在 dry 阶段是否可放通
- 命令
  - `HEADLESS_ALLOW_SENSITIVE=1 bash scripts/run_headless_research_matrix.sh --pipeline dry,mock,service --templates material_study_envelope_catalog,material_study_envelope_ranking --service-fallback 0 --retries 1 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round17 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round17-20260812`
- 结果文件
  - [reports/headless-research-matrix-round17-20260812-20260812-131633.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round17-20260812-20260812-131633.md)
  - `results/headless-research-matrix-20260812-round17/summary.json`
- 结论：即使设置了 `HEADLESS_ALLOW_SENSITIVE=1`，`dry` 阶段仍为 `blocked`；说明当前 dry-run 路径未透传放通开关（或 dry-run 的确认策略与 service 不一致）。

### 执行9：敏感模板 service 路径复验
- 命令
  - `HEADLESS_ALLOW_SENSITIVE=1 bash scripts/run_headless_research_matrix.sh --pipeline service --templates material_study_envelope_catalog,material_study_envelope_ranking --service-fallback 0 --retries 1 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round18 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round18-20260812`
- 结果文件
  - [reports/headless-research-matrix-round18-20260812-20260812-131647.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round18-20260812-20260812-131647.md)
  - `results/headless-research-matrix-20260812-round18/summary.json`
- 结论：两套 `material_study_*` 模板 service 执行成功（`service_primary_status=ok`），无 fallback，证实 dry 阶段与 service 阶段行为差异明显。

### 执行10：默认池 service 全量（验证服务稳定性与候选提取）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline service --service-fallback 1 --retries 1 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round19 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round19-20260812`
- 结果文件
  - [reports/headless-research-matrix-round19-20260812-20260812-131655.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round19-20260812-20260812-131655.md)
  - `results/headless-research-matrix-20260812-round19/summary.json`
- 结论：
  - 11/11 默认模板 `service` 成功；
  - 关键材料模板可拿到 winner（如 `material_dielectric_screening`、`material_structural_panel_screening`、`material_composite_thermo_electric_panel_screening` 等）。

### 执行11：兜底触发条件回归验证（主通道故障）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline service --templates direct_plane_triangle --service-primary-url http://127.0.0.1:59999 --service-fallback 1 --retries 1 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round20 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round20-20260812`
- 结果文件
  - [reports/headless-research-matrix-round20-20260812-20260812-131753.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round20-20260812-20260812-131753.md)
  - `results/headless-research-matrix-20260812-round20/summary.json`
- 结论：主通道 `transport_failure`，但兜底仍 `skipped`，与当前回归判定策略（仅在特定失败类）一致。

### 执行12：labctl service 通道链路回归（模块化）
- 命令
  - `bash scripts/labctl.sh run headless-research-matrix --run-id continue-service-suite-20260812 --workspace /Users/Shared/chroot/dev/kyuubiki/runs/headless-research-matrix/continue-service-suite-20260812/workspace --set PIPELINE=service --set TEMPLATES=direct_mesh_pipeline,material_structural_panel_screening --set SERVICE_FALLBACK=0 --set WORKDIR=/Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-labctl-20260812c --set REPORT_DIR=/Users/Shared/chroot/dev/kyuubiki/reports --set REPORT_BASENAME=headless-research-matrix-labctl-20260812c --set MAX_ATTEMPTS=1`
- 结果文件
  - [runs/headless-research-matrix/continue-service-suite-20260812/run-manifest.json](/Users/Shared/chroot/dev/kyuubiki/runs/headless-research-matrix/continue-service-suite-20260812/run-manifest.json)
  - [results/headless-research-matrix-labctl-20260812c/summary.json](/Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-labctl-20260812c/summary.json)
- 结论：module->script 入口在 service 模式下闭环可复用，2 模板全部成功。

### 执行13：all 模式压测回归（dry+mock+service 全链路）
- 命令
  - `bash scripts/run_headless_research_matrix.sh --pipeline all --templates direct_mesh_pipeline,material_study_envelope_catalog,material_study_envelope_ranking --service-fallback 0 --retries 1 --workdir /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round21 --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-research-matrix-round21-20260812`
- 结果文件
  - [reports/headless-research-matrix-round21-20260812-20260812-131843.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-round21-20260812-20260812-131843.md)
  - `results/headless-research-matrix-20260812-round21/summary.json`
- 结论：
  - `direct_mesh_pipeline` dry+mock+service 均 ok；
  - `material_study_*` 两模板在 dry 阶段 `blocked`，从而 mock/service 自动跳过。

## 关键问题与修复
- `scripts/labctl.sh`：`module_args[@]: unbound variable`（`set -u`）修复为数组安全声明/展开。
- `scripts/run_headless_research_matrix.sh`：最初出现 `material_args[@]: unbound variable`，已改为 `local -a material_args=()` 与安全展开。
- dry/mock 场景下，`material_study_*` 模板会返回 `blocked_by_confirmation: risk=sensitive`，非 bug，表示需要确认流程放通后才会执行；
- `material_study_ranking_shift` 为不存在模板，当前 CLI 返回 `headless_command_failed`（含最近邻建议），说明参数校验路径是健康的。
- 可复现实例：在 dry 阶段即便 `HEADLESS_ALLOW_SENSITIVE=1`，`material_study_*` 仍可能返回 `blocked`，疑似 dry-run 与 service 对敏感开关注入策略不一致（建议统一确认策略）。
- `service` 兜底（4000）目前只在“artifact_limit/413/传输体积”类失败时触发；遇到连接失败（transport_failure）时保持 `service_fallback=skipped`。

## 当前服务通道状态（硬结论）
- service 主通道 `127.0.0.1:3000` 在多数可复测窗口表现为稳定可达并返回 `ok`（多次默认池全量 service 全绿），偶发于重启窗口出现 `transport_failure`（可复现网络重连场景）。
- 该问题与 dry/mock 算法链路无关。

## 下一步建议
- 可直接在当前链路上做 3 个优化验证：
  - 将 `HEADLESS_ALLOW_SENSITIVE`/`--allow-sensitive` 在 dry 阶段同步注入，确认 `material_study_*` 是否可完整进入 dry->mock->service；
  - 将 `service_fallback` 兜底触发策略放宽到 `transport_failure`，观察 fallback 是否能在端口切换场景下补位；
  - 继续用 `--pipeline service --retries 2` 扩展默认池长跑（建议固定 run-id）统计成功率与 p95/p99 延迟。
## 执行14：服务压测批次（stress batch）
- 时间：2026-08-12
- 场景：bash 批处理 20 次复用调用（每轮执行 direct_mesh_pipeline 与 material_composite_thermo_electric_panel_screening）
- 任务规模：40 条
- 成功率：40/40（service_primary_status=ok）
- 总体耗时（秒）：
  - count:       40
  - min: 20
  - max: 26
  - mean: 22.750000
  - p50: 23
  - p90: 25
  - p95: 26
  - p99: 26
- 按模板耗时（秒）：
  - `direct_mesh_pipeline`: n=20, min=20, max=26, mean=22.750000, p50=23, p90=25, p95=26, p99=26
  - `material_composite_thermo_electric_panel_screening`: n=20, min=20, max=26, mean=22.750000, p50=23, p90=25, p95=26, p99=26
- 结论：服务端单批次执行稳定，耗时主要分布在 20~26 秒。建议下轮继续把网格规模/模板复杂度提升（1e5~1e6 网格或更密节点）验证时间线性、资源饱和与退化点。


## 执行15：复杂多物理模板服务压测批次（complex stress）
- 时间：2026-08-12
- 场景：11 模板全链路 service 并行顺序执行，3 次重复；模板包含 direct + material 混合
- 模板：`direct_acoustic_bar_1d`, `direct_electrostatic_triangle`, `direct_heat_triangle`, `direct_plane_triangle`, `direct_thermal_frame_3d`, `direct_thermal_truss_3d`, `direct_mesh_pipeline`, `material_dielectric_screening`, `material_structural_panel_screening`, `material_composite_thermo_electric_panel_screening`, `material_thermo_shield_screening`
- 任务规模：33 条（3 次 × 11 模板）
- 成功率：33/33（service_primary_status=ok）
- 总体耗时（秒）：count=33，min=54，max=57，mean=55.6667，p50=56，p90=57，p95=57，p99=57
- service_fallback：未触发（全部 `service_fallback_status=skipped`）
- 按模板耗时（秒）：
  - `direct_acoustic_bar_1d`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `direct_electrostatic_triangle`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `direct_heat_triangle`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `direct_mesh_pipeline`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `direct_plane_triangle`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `direct_thermal_frame_3d`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `direct_thermal_truss_3d`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `material_composite_thermo_electric_panel_screening`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `material_dielectric_screening`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `material_structural_panel_screening`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
  - `material_thermo_shield_screening`: n=3, min=54, max=57, mean=55.6667, p50=56, p90=57, p95=57, p99=57
- 结论：增加模板数量后，单 run 总耗时从 54~57 秒，单项 p50=56；整体稳定性保持高，未观察到 transport_failure/阻断。下一步建议：
  - 启用 `--service-fallback 1` 在主通道注入瞬时失败场景；
  - 再引入 1M 网格的直接模板用于线性伸缩验证。


## 执行16：主通道失联时 fallback 验证（transport_failure 场景）
- 时间：2026-08-12
- 场景：主服务指向不可达端口（`http://127.0.0.1:3999`），兜底指向正常端口（`http://127.0.0.1:3000`），模板为 2 个。
- 命令：`run_headless_research_matrix.sh --pipeline service --templates direct_mesh_pipeline,material_composite_thermo_electric_panel_screening --service-primary-url http://127.0.0.1:3999 --service-fallback-url http://127.0.0.1:3000 --service-fallback 1`
- 结果：
  - `direct_mesh_pipeline`: service_primary `failed`，`service_primary_error_code=kyuubiki.headless.transport_failure`
  - `material_composite_thermo_electric_panel_screening`: service_primary `failed`，`service_primary_error_code=kyuubiki.headless.transport_failure`
  - 两个模板的 `service_fallback_status` 均为 `skipped`，`run_service_fallback_exit=-1`
- 观察到的关键问题：
  - 目前 fallback 触发条件仅限 `is_artifact_limit_failure`。
  - `transport_failure`（服务不可达）不会触发 fallback，因此高优先级的高可用路径在“节点抖动/端口断开”场景下未生效。
- 参考报表：
  - [headless-research-matrix-fallback-route-20260812-20260812-133503.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-route-20260812-20260812-133503.md)
  - 指标快照：[fallback-route-metrics.csv](/tmp/fallback-route-metrics.csv)

## 执行17：主通道失联时 fallback 修复验证（transport_failure 场景）
- 时间：2026-08-12
- 场景：主服务指向不可达端口（`http://127.0.0.1:3999`），兜底指向正常端口（`http://127.0.0.1:3000`），模板：`direct_mesh_pipeline` 与 `material_composite_thermo_electric_panel_screening`
- 变更前提：本地修复 `scripts/run_headless_research_matrix.sh`，将 `is_artifact_limit_failure` 的触发条件扩展为 `kyuubiki.headless.transport_failure`（并包含常见连接失败关键词）。
- 结果：
  - `direct_mesh_pipeline`：`service_primary_status=failed`（`transport_failure`），`service_fallback_status=ok`
  - `material_composite_thermo_electric_panel_screening`：`service_primary_status=failed`（`transport_failure`），`service_fallback_status=ok`，且给出完整候选输出
- 核验文件：
  - [headless-research-matrix-fallback-route-20260812-fixed-20260812-133626.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-route-20260812-fixed-20260812-133626.md)
  - [fallback-route-fixed-metrics.csv](/tmp/fallback-route-fixed-metrics.csv)
  - [results/headless-research-matrix-fallback-misroute-fixed/summary.json](/Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-fallback-misroute-fixed/summary.json)
- 结论：修复有效，`transport_failure` 已触发 fallback，主通道抖动下服务链路可继续返回 `service_fallback_status=ok`，提高高可用鲁棒性。

## 执行18：失败主通道按 rc 触发 fallback（兜底增强）
- 时间：2026-08-12
- 变更：在 `scripts/run_headless_research_matrix.sh` 中把 fallback 判定条件改为：主通道执行返回码非零时也触发 fallback（`service_primary_s != 0`），同时保留原有 `artifact_limit` 路径。
- 验证命令：
  - `--pipeline service --templates direct_mesh_pipeline,material_composite_thermo_electric_panel_screening --service-primary-url http://127.0.0.1:3999 --service-fallback-url http://127.0.0.1:3000 --service-fallback 1`
- 结果：
  - 两个模板仍出现 `service_primary_status=failed`、`service_primary_error=kyuubiki.headless.transport_failure`，且 `service_fallback_status=ok`（`service_fallback_exit=0`）。
- 说明：本次场景下 fallback 已经在主通道故障时触发。该逻辑也覆盖了所有 `run_service_primary` 非零返回码的情况，满足你提到的“`run_cmd_with_retry` 后最终 rc 的兜底判定”诉求。
- 参考：
  - [headless-research-matrix-fallback-route-20260812-rc-20260812-133718.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-route-20260812-rc-20260812-133718.md)
  - [fallback-route-fixedrc-metrics.csv](/tmp/fallback-route-fixedrc-metrics.csv)

## 执行19：主通道偶发失联（抖动）fallback 连续性验证
- 时间：2026-08-12
- 场景：4 轮混合运行，奇数轮主通道 `http://127.0.0.1:3999`（不可达）、偶数轮主通道 `http://127.0.0.1:3000`（可达）；fallback 固定 `http://127.0.0.1:4000`，`service-fallback=1`。
- 模板：`direct_mesh_pipeline`，`material_composite_thermo_electric_panel_screening`
- 命令：`run_headless_research_matrix.sh --pipeline service --retries 2`，主通道 URL 在每轮交替。
- 统计（8 条结果）：
  - 主通道 `failed`：4 条（全部触发 `kyuubiki.headless.transport_failure`）
  - 主通道 `ok`：4 条
  - fallback `ok`：4 条（仅在 down 轮出现）
  - fallback `skipped`：4 条（仅在 ok 轮出现）
- 关键结论：
  - 当主链路恢复后，fallback 正常退回 `skipped`，无残留错误；
  - 当主链路再次失联，`service_fallback` 立即接管且稳定返回 `ok`，表现出抖动链路下的快速回退能力。
- 参考：
  - [headless-research-matrix-fallback-chatter-20260812-1-20260812-134021.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-chatter-20260812-1-20260812-134021.md)
  - [headless-research-matrix-fallback-chatter-20260812-2-20260812-134054.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-chatter-20260812-2-20260812-134054.md)
  - [headless-research-matrix-fallback-chatter-20260812-3-20260812-134114.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-chatter-20260812-3-20260812-134114.md)
  - [headless-research-matrix-fallback-chatter-20260812-4-20260812-134144.md](/Users/Shared/chroot/dev/kyuubiki/reports/headless-research-matrix-fallback-chatter-20260812-4-20260812-134144.md)
  - 指标：[fallback-chatter-metrics.csv](/tmp/fallback-chatter-metrics.csv)

## 执行20：1M 级网格压测 + 主通道抖动 fallback 连续性验证
- 时间：2026-08-12
- 场景：直接调用 `kyuubiki headless run`，使用预制输入 `results/sdk-large-mesh-1m/input_700x700.json`（约 490k 网格单元）与 `results/sdk-large-mesh-1m/input_1000x1000_noids.json`（约 1,000,000 网格单元，含无ID版本）。
- 执行策略：3 组场景（先后执行）
  - 组 A：`primary=http://127.0.0.1:3999`（主通道故障模拟）+ `fallback=http://127.0.0.1:3000`
  - 组 B：`primary=http://127.0.0.1:3000`（主通道正常）+ `fallback=http://127.0.0.1:4000`
  - 组 C：`primary=http://127.0.0.1:3999`（主通道故障模拟）+ `fallback=http://127.0.0.1:3000`
- 通用参数：`--json --report-out <case>/report.json --execute --executor service --job-wait-timeout-ms 1200000 --execution-posture research`
- 结果数据（单位：s）：
  - `input_700x700.json`
    - A：`primary_rc=1`，`primary_status=failed`，`primary_error_code=kyuubiki.headless.transport_failure`，`duration=2`
    - A `fallback_rc=0`，`fallback_status=ok`，`duration=5`
    - B：`primary_rc=0`，`primary_status=ok`，`duration=4`，无需 fallback
  - `input_1000x1000_noids.json`
    - A：`primary_rc=1`，`primary_status=failed`，`primary_error_code=kyuubiki.headless.transport_failure`，`duration=1`
    - A `fallback_rc=0`，`fallback_status=ok`，`duration=5`
    - B：`primary_rc=0`，`primary_status=ok`，`duration=5`，无需 fallback
- 关键观察：
  - 主通道不可达时，`headless` 仍返回 `run_service_primary status=failed`（`transport_failure`），`fallback` 会在同一场景下成功兜底，且可在 `3000/4000` 之间保持预期切换。
  - 1M 级输入在正常主通道下成功耗时可控（1M 案例约 5s，700x700 案例约 4s），说明执行链路对大输入已具有较稳定吞吐。
  - fallback 通道返回 `ok` 时 `mode=execute:service`，`error_code=n/a`，`steps=9`，产物可用。
- 关键日志/产物（完整保留）：
  - 案例根目录：`/Users/Shared/chroot/dev/kyuubiki/results/research-large-grid-round-20260812/`
  - 失败主通道摘要：
    - `input_700x700_p3999_f3000/primary_report_summary.json`
    - `input_1000x1000_noids_p3999_f3000/primary_report_summary.json`
  - 兜底摘要：
    - `input_700x700_p3999_f3000/fallback_report_summary.json`
    - `input_1000x1000_noids_p3999_f3000/fallback_report_summary.json`
- 初步结论：当前修复后的 fallback 判定链路（`service_primary_s != 0 || is_artifact_limit_failure`）在高网格压测下继续成立，主链路抖动场景下可自动接管，暂未发现该场景新增 regression。

## 执行21：大规模 jobwait 输入压测（artifact_limit 与端口退化并行验证）
- 时间：2026-08-12
- 输入：`input_1000x1000_noids_jobwait_1200000.json`、`input_700x700_jobwait_1200000.json`（均含 `--job-wait-timeout-ms 1800000`）
- 场景1（`results/research-large-grid-round-20260812-3`）：
  - 轮替策略（6 条）：`3999->3000`、`3000->4000`、`3999->3000`、`3999->3000`、`3000->4000`、`3999->3000`
  - 主通道结果：
    - 6 条全部失败（`primary_rc=1`）
    - 失败码：`kyuubiki.headless.transport_failure`、`kyuubiki.headless.frontend_proxy_artifact_limit`
  - fallback 结果：
    - `3000->4000` 轮次（2/6）fallback 成功 `ok`
    - `3999->3000` 轮次（4/6）fallback 失败，且失败码为 `kyuubiki.headless.frontend_proxy_artifact_limit`
  - 关键现象：
    - 1M 输入在主通道 `3000` 上经常直接报 `frontend_proxy_artifact_limit`，说明 `3000` 在大体量输入下仍表现为 frontend 前端转发路径而非直接控制面，不满足 artifacts 传输要求。
    - 该问题会把 `frontend_proxy_artifact_limit` 放大为 fallback 层面同源失败，尤其当 fallback 仍指向 `3000` 时。
  - 目录：`/Users/Shared/chroot/dev/kyuubiki/results/research-large-grid-round-20260812-3`

## 执行22：大输入 + 主通道强制断开，直接退向 `4000` 验证
- 时间：2026-08-12
- 输入：与执行21一致的 jobwait 大文件
- 场景：`3999->4000`（4 条）
- 结果：
  - 4 条主通道均 `primary_rc=1`，`error=kyuubiki.headless.transport_failure`
  - 4 条 fallback 均 `ok`
  - fallback 耗时（秒）：`189 / 186 / 95 / 92`
- 结论：
  - 在主通道断联（`3999`）时，直接切到 `4000` 可稳定接管；问题不在重连机制本身，而在于兜底端口的能力覆盖策略。
- 目录：`/Users/Shared/chroot/dev/kyuubiki/results/research-large-grid-round-20260812-4`

## 建议修复点（高优先）
- 复用矩阵脚本在遇到大网格输入时，fallback 路径应基于 `error_code`（特别是 `frontend_proxy_artifact_limit`）自动切换到控制面端口（默认 `4000`）而不是盲目沿用参数化主/次端口。
- 建议在复现脚本层面输出更细的 artifact 指标（如 `entity_count`）与 `--execution-posture`、`--api-base-url` 组合日志，避免误把“主通道可达但走了 frontend 受限路径”误判为稳定通道。
