# kyuubiki 复杂场景研发轮次（2026-08-12）

## 1) 环境与基础状态核对
- 执行时间: 2026-08-12
- 仓库: `/Users/Shared/chroot/dev/kyuubiki`
- 核心目标: 继续高复杂场景探索（`mesh` / `多物理` / `材料筛选`），并补充旧问题复现证据。
- 备注: 按你要求，本轮不再把“旧问题复测后恢复”误判为新缺陷，只记录当前有效现象。

## 2) 本地服务启动与端口问题（确定可复现）
命令:
- `./scripts/kyuubiki status`
- `./scripts/kyuubiki start`（以及 `KYUUBIKI_AGENT_ENDPOINTS=127.0.0.1:50001,127.0.0.1:50002 ./scripts/kyuubiki start`）

结果:
- 本地 `status` 显示: orchestrator/frontend/agent 全部 stopped。
- `start` 在等待端口 5001（/50001）时超时并返回：
  - `agent error: failed to bind 127.0.0.1:5001: Operation not permitted (os error 1)`
  - 或替换为 50001 时同样复现。

结论:
- 这不是代码执行失败而是本地环境/沙箱对监听 socket 的权限/能力限制。
- 这会直接阻断 service-based 真实执行链路（headless 的 service executor / orchestrator 端口打通）。

## 3) 远端编排服务现象（kyuubiki-lab）
命令:
- `ssh kyuubiki-lab "cd ~/kyuubiki && ./scripts/kyuubiki status"`
- `ssh kyuubiki-lab "cd ~/kyuubiki && ./scripts/kyuubiki start"`

结果:
- `status` 起步可见 frontend 运行，agent[5001] running，agent[5002] 需要补起。
- `start` 会拉起 orchestrator，但短时间后日志出现：
  - `SIGTERM received - shutting down`
  - `Mix ... trying to run :kyuubiki_web on Elixir v1.14.0 but ... supports only Elixir ~> 1.19`

结论:
- orchestrator 被系统自检的 Elixir 版本要求强制终止（远端环境用的是 `Elixir 1.15.7`）。
- 这是当前远端 service-run 路径的高优先级阻断条件。

## 4) 复杂模板复合跑（headless mock，可重复）
使用脚本: `./scripts/run-headless-template-matrix.sh`

命令:
- `./scripts/run-headless-template-matrix.sh --mode both --executor mock --templates direct_plane_quad,direct_mesh_pipeline,material_dielectric_screening,material_structural_panel_screening,material_composite_thermo_electric_panel_screening --workdir /tmp/kyuubiki-template-matrix-complex --report-dir /Users/Shared/chroot/dev/kyuubiki/reports --report-basename headless-template-matrix-complex`

输出:
- `summary`: `/tmp/kyuubiki-template-matrix-complex/summary.json`
- 报告: `/Users/Shared/chroot/dev/kyuubiki/reports/headless-template-matrix-complex-20260812-101117.md`

全部 5 个模板返回 `0` 退出码（`init/validate/plan/render/run_dry/run_exec` 全通过），并且 dry/execute 都为 `ok`。

结论:
- SDK headless 流程本体在 mock 执行层是稳定的。
- 当前卡点主要在 service 运行链路（本地端口权限 + 远端 Elixir 版本匹配），而不是模板 SDK 语义。

## 5) 风险分级建议（按修复优先级）
- P0（阻断）: 本地监听端口被权限拦截，无法在本机运行真实 service 运行链路。
- P1（阻断）: 远端 `kyuubiki-lab` orchestrator 依赖 Elixir 版本不满足 ~>1.19。
- P2（偏差）: 远端 `kyuubiki-lab` 的 `scripts/kyuubiki --help` 当前未显式支持 `headless` 命令，且与本地 dev 偏新版本不一致（可能是脚本镜像版本差异）；应同步后再继续 service/headless 对比。

