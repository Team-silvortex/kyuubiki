# 2026-08-10 20:50 头less研发复测（新版模板矩阵）

## 本轮目标
- 使用 `dev` 同步后的 headless SDK 做“更实战”的研发场景覆盖。
- 覆盖电磁/热/力/材料/网格等多物理模板：`init -> validate -> plan -> run`（dry-run）
- 额外加命令边界测试：runtime-style、execution-posture、api-base-url。

## 证据环境
- 工作目录：`/Users/Shared/chroot/dev/kyuubiki`
- 输出临时工作区：`/private/tmp/kyuubiki_round_latest`

## 模板流水线结果（本地 mock/dry-run）
以下模板全部经过 `headless init -> validate -> plan -> run --json`，`run` 未带 `--execute`（预演模式）：

| 模板 | 初始化 | 校验 | plan | run 模式 | 执行步数 | 结果 |
|---|---|---|---|---|---:|---|
| `solve_wait_result` | OK | OK | OK | `dry_run` | 4 | ok |
| `direct_electrostatic_triangle` | OK | OK | OK | `dry_run` | 3 | ok |
| `direct_heat_triangle` | OK | OK | OK | `dry_run` | 3 | ok |
| `direct_thermal_frame_3d` | OK | OK | OK | `dry_run` | 3 | ok |
| `direct_acoustic_bar_1d` | OK | OK | OK | `dry_run` | 3 | ok |
| `direct_mesh_pipeline` | OK | OK | OK | `dry_run` | 3 | ok |
| `direct_truss_3d` | OK | OK | OK | `dry_run` | 3 | ok |
| `material_composite_thermo_electric_panel_screening` | OK | OK | OK | `dry_run` | 9 | ok |

## execute mock 覆盖（无服务后端）
- `direct_mesh_pipeline`：`headless run <workflow> --execute --executor mock --json` -> `status: ok`, `mode: execute:mock`, `executed_step_count: 3`
- `material_composite_thermo_electric_panel_screening`：`--execute --executor mock --json` -> `status: ok`, `mode: execute:mock`, `executed_step_count: 9`
- 这两组都未出现 `requires_confirmation`

## 查询能力验证（`headless templates --json`）
- `--query electrothermal`：`template_count = 1`
- `--tag multiphysics`：`template_count = 1`
- `--category mechanical`：`template_count = 12`
- `--runtime browser_only`：`template_count = 1`
- `--category garbage`：`template_count = 0`

## 真实服务姿态测试（对照）
- `--execution-posture research --executor service --api-base-url http://127.0.0.1:4000`：
  - `status: failed`
  - `error_code: kyuubiki.headless.transport_failure`
  - message: 无法连通 127.0.0.1:4000 (`Operation not permitted`)
- 这是运行环境连通性问题，不是模板定义问题。

## 边界与可修问题（仅列本轮新增）
1. `headless init --runtime-style hybrid --template direct_truss_3d` 返回 `unknown headless template`。
   - 真实模板 `direct_truss_3d` 在列表里存在。
   - 错误文本中虽给出近似候选，包含该模板本体，提示存在“模板 ID 在 runtime 筛选条件下被隐藏后仍走未知模板分支”的语义歧义。
   - 严重度：P2（可用性/诊断性）
2. `headless run ... --executor mock --execution-posture research` 会返回结构化错误：`research execution requires --executor service; mock cannot provide a no-mock execution guarantee`
   - 校验行为正确，但建议文案与命令参数约束提示可更统一。
3. `--api-base-url` 对非法地址（HTTPS / 含路径）会返回明确错误文本，属预期校验行为。

## 结论
- 本轮实研场景在 mock/dry-run 与 execute:mock 下稳定通过。
- 主要待优化点集中在 `runtime-style` 与 `template` 同时使用时的提示一致性。
