# 复杂场景研发回归报告（2026-08-12）

- 时间: 2026-08-12 10:07:00
- 评估目标: 更复杂场景组合验证（链式闭环 + 服务端口回退 + 大网格回退 + 多模板矩阵）
- 运行基础目录: `/Users/Shared/chroot/dev/kyuubiki`

## 一、本轮执行清单

### 1) 链式研究闭环

- 脚本: `research-scripts/run_chain_next_regression.sh`（拷贝自 `/Users/Shared/chroot/research/kyuubiki-playground/scripts`）
- 参数:
  - `KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki`
  - `CHAIN_ROUNDS=3`
  - `STUDY=dielectric-screening`
  - `SYNC_SDK_FROM_DEV=0`
- 报告: `/Users/Shared/chroot/dev/kyuubiki/reports/chain-next-regression-report.md`
- 结果摘要:
  - `baseline` 与 `replay` 都成功（`round_count=3`）
  - `chain-next` 在固定输入下具备复现性：`candidate_input_fingerprint` 一致
  - 预期失败用例均按预期返回（`rounds=0`、bad study、missing input）

### 2) 服务端口回退矩阵（3000/4000）

- 脚本: `scripts/run_service_port_matrix.sh`
- 参数:
  - `KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki`
  - `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki`
  - `FORCE_FALLBACK_4000=1`
- 报告: `/Users/Shared/chroot/dev/kyuubiki/reports/service-matrix-port-rotation-20260812-100129.md`
- 结果摘要（3 个用例均失败）：
  - 输入 `service_3000` / `service_4000` 的 `report.status` 全部为 `failed`
  - `error_code` 为 `headless_execution_failed`
  - 关键错误是 `Operation not permitted (os error 1)`，表明服务端口不可达（连接控制面失败）
  - `body-limit` 回退签名未触发（`service_3000.body-limit-signature: 0`）

### 3) 大网格自动回退（1M 规模）

- 脚本: `scripts/run_large_mesh_auto_fallback.sh`
- 参数: 使用默认回退阈值 + `KYUUBIKI_REPO_ROOT=/Users/Shared/chroot/dev/kyuubiki`, `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki`
- 报告: `/Users/Shared/chroot/dev/kyuubiki/reports/large-mesh-auto-fallback-20260812-100140.md`
- 结果摘要:
  - 两个输入均检测到 `frontend_proxy_artifact_limit`，触发 4000 回退
  - 4000 回退后继续失败，`error_code=kyuubiki.headless.transport_failure`
  - 关键失败: `failed to connect to 127.0.0.1:4000 for model artifact upload`

### 4) 多模板闭环矩阵

- 脚本: `scripts/run_headless_template_matrix.sh`
- 参数:
  - `TEMPLATE_REGRESSION_TS=20260812-102500`
  - `HEADLESS_ROUNDS=1`
  - `HEADLESS_START_VOLTAGE=900`
  - `HEADLESS_MIN_VOLTAGE=900`
  - `HEADLESS_MAX_VOLTAGE=2500`
- 报告: `/Users/Shared/chroot/dev/kyuubiki/reports/headless-template-matrix-20260812-102500.md`
- 结果摘要:
  - 覆盖 33 个模板（`discovered`）
  - 所有模板在 execute 阶段前后均因服务不可达退化为失败；
  - `direct_*` 与 `material_*` 中，大多数表现为 `fail`；`material_study_envelope_*` 因 `blocked` 跳过 execute 的边界用例。

## 二、关键问题与证据

### P0 候选（阻塞研发流程）

1. `run_headless_template_matrix.sh` 假设 `run_headless_workflow_regression.sh` 位于 `$WORKSPACE_DIR/scripts`。
- 在 `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki` 时首次运行报：`bash: .../scripts/run_headless_workflow_regression.sh: No such file or directory`
- 结果导致整轮矩阵表将大量条目标记为 `fail`，掩盖了真实执行状态。
- 建议: 在脚本中用与本文件同目录或 `$KYUUBIKI_REPO_DIR/scripts` 定位被调脚本，避免 workspace 与脚本仓库路径耦合。

2. 服务端口相关：当前环境下执行面无法在 127.0.0.1:3000 / 4000 建立连接，出现统一 `transport_failure`。
- 该问题会把模板矩阵、端口矩阵及大网格回退全部推入失败态，不反映算法正确性本身。
- 建议: 增加预检 `healthcheck`，若控制面不可达则直接失败分类为“环境退化”，并在报告中独立标记。

### P1/P2 候选（可见性与可观测性）

3. `run_headless_template_matrix.sh` 在被调脚本失败时把每个步骤直接写成 `fail`。
- 缺失真实分阶段状态（即便 init/validate/plan/render 已成功），不利于定位。
- 建议: 读取并上报 `run_headless_workflow_regression.sh` 真实 `headless-loop` 状态文件，即便最终命令失败也逐字段上报。

4. 大网格回退脚本对 4000 失败的可恢复性边界较弱。
- 已正确触发 `frontend_proxy_artifact_limit` 并回退，但 `4000` 不可达时无法区分“回退目标不可达”与“回退后仍解析失败”。
- 建议: 增加失败分类与建议动作（例如：优先校验控制面健康/端口代理配置）。

## 三、可复现命令（本轮）

- `KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki DEV_KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki CHAIN_ROUNDS=3 STUDY=dielectric-screening SYNC_SDK_FROM_DEV=0 bash /Users/Shared/chroot/research/kyuubiki-playground/scripts/run_chain_next_regression.sh`
- `KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki FORCE_FALLBACK_4000=1 python3 /Users/Shared/chroot/research/kyuubiki-playground/scripts/run_service_port_matrix.sh`
- `KYUUBIKI_REPO_ROOT=/Users/Shared/chroot/dev/kyuubiki WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki python3 /Users/Shared/chroot/research/kyuubiki-playground/scripts/run_large_mesh_auto_fallback.sh`
- `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki TEMPLATE_REGRESSION_TS=20260812-102500 HEADLESS_ROUNDS=1 HEADLESS_START_VOLTAGE=900 HEADLESS_MIN_VOLTAGE=900 HEADLESS_MAX_VOLTAGE=2500 bash /Users/Shared/chroot/research/kyuubiki-playground/scripts/run_headless_template_matrix.sh`
