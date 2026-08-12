# Kyuubiki 研发探索轮报（2026-08-12）

## 目标
- 用同一版本（本地）继续补齐复杂研发场景。
- 重点覆盖：
  - `workflow_submit_monitor` 敏感步骤的确认门控。
  - service 执行链路在端口 3000/4000 下的回退行为。
  - 服务启动与 transport 层错误的真实边界。

## 本轮执行清单
1. 远端启动尝试（覆盖 PATH/版本变量）：
   - `ssh kyuubiki-lab '... PATH=$HOME/.local/elixir-1.15.7-otp-25/bin:$PATH ... ./scripts/kyuubiki start'`
2. 读取 orchestrator 日志尾部确认启动失败上下文。
3. `workflow_submit_monitor` 只用 mock + execute：
   - `scripts/run-headless-template-matrix.sh --template workflow_submit_monitor --executor mock --mode execute`
4. 手工加 `--allow-sensitive` 直接验证 mock 下 unblock：
   - `scripts/kyuubiki headless run ... --allow-sensitive --execute --executor mock`
5. service 执行回归：
   - `HEADLESS_TEMPLATE=workflow_submit_monitor HEADLESS_ALLOW_SENSITIVE=1 ... ./scripts/run_headless_workflow_regression.sh`
6. service 端口矩阵复测（3000→4000）：
   - `python3 scripts/run_service_port_matrix_copy.sh`

## 主要结果

### 1) 远端服务启动依然不健康
- 覆盖 `PATH`/`MIX_HOME` 后，agent 能被拉起，但 orchestrator 日志在启动即将尾部出现：
  - `SIGTERM received - shutting down`
  - `Mix) You're trying to run :kyuubiki_web on Elixir v1.14.0 but it has declared ... ~> 1.19`
  - `Mix) Could not find an SCM for dependency :jason...`
- 这说明当前运行环境仍有 Elixir 版本与依赖初始化一致性问题。
- 结合 `scripts/kyuubiki status` 显示 orchestrator 常为 `stopped`，service/API 层在本次测试窗口不可用。

### 2) sensitive 工作流门控行为（mock 与 allow-sensitive）
- `workflow_submit_monitor` 在 mock 下不带敏感开关会被直接 block：
  - `run_exec` 状态 `blocked`
  - `blocked_by_confirmation.index=1, risk=sensitive`
  - `execution_summary.job_ids=["job_001"]`，执行步数 0。
- 加 `--allow-sensitive` 后，mock 可完整执行：
  - `status=ok`
  - `mode=execute:mock`
  - `executed_step_count=3`
  - `workflow_submit_catalog/job_wait/result_fetch` 均显示 `executed`。

### 3) service 执行与 transport 错误（复现清晰）
- `run_headless_workflow_regression.sh`（`HEADLESS_TEMPLATE=workflow_submit_monitor`, `HEADLESS_ALLOW_SENSITIVE=1`）在 `run_exec_round_1` 失败：
  - `failed to connect to 127.0.0.1:3000 for service request ... Operation not permitted (os error 1)`
  - `kyuubiki.headless.transport_failure`, `stage=transport`, `step_index=1`
- 同类现象在端口矩阵脚本复测中也一致出现：
  - case: `large_700x700`, `large_1000x1000_noids`, `small_direct_heat_triangle`
  - 3000 侧所有 `service_3000` 结果为 shell status `1`、run status `failed`，且 first step transport 失败。
- 由于未触发 body-limit 签名，`run_service_port_matrix_copy.sh` 未尝试 4000 回退（`fallback skipped`）。

## 结论（本轮）
1. 当前环境下 **service 模式从头到尾不可用**，根因定位在控制面启动/连通链路，而不是模板/工作流本身。
2. mock 下 `workflow_submit_monitor` 的敏感步骤控制门控行为清晰：
   - 无 `--allow-sensitive` => block；
   - 有 `--allow-sensitive` => 可执行。
3. `run_service_port_matrix_copy.sh` 的 fallback 目前只按“artifact/body limit”触发；若未来要覆盖纯 transport 失联场景，可考虑把该规则拓展为在 `kyuubiki.headless.transport_failure` 时也尝试控制面 4000。

## 本轮新增/待修复问题清单
- [P1] 远端 orchestrator 受环境限制/版本约束退出，导致 service 127.0.0.1:3000 连通失败，service 执行全部受阻。
- [P3] 端口回退脚本对 transport 失败不触发 4000 重试，可能掩盖同类连通问题的二次验证路径。

## 输出文件
- `/tmp/kyuubiki-template-matrix-workflow-submit/summary.json`
- `/tmp/kyuubiki-template-matrix-workflow-submit/workflow_submit_monitor/exec-report.json`
- `/tmp/kyuubiki-template-matrix-workflow-submit/workflow_submit_monitor/exec-allow-sensitive-report.json`
- `/Users/Shared/chroot/dev/kyuubiki/headless-loop/`
- `/Users/Shared/chroot/dev/kyuubiki/results/service-matrix-port-rotation-20260812-1022/`
- `/Users/Shared/chroot/dev/kyuubiki/reports/service-matrix-port-rotation-20260812-1022.md`
- `/Users/Shared/chroot/dev/kyuubiki/reports/kyuubiki-research-round-20260812-service-mock-vs-live.md`
