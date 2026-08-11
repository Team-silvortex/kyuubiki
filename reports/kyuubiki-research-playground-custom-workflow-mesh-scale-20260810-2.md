# kyuubiki 研究记录：自定义 workflow 直参压测（round-20260810-2）

## 测试目标
- 验证：在不改 SDK 的情况下，是否可直接基于 `headless init` 产物（workflow json）进行参数化开发。
- 验证：不同规模网格（`elements`）与不同执行器（`mock` / `service` / `hybrid`）在 headless 链路下的行为。
- 验证：服务端路径是否可用。

## 环境与脚本
- 路径：`/tmp/kyuubiki-direct-mesh-init.json`（由 `scripts/kyuubiki headless init --template direct_mesh_pipeline` 生成）
- 实验脚本：`/tmp/kyuubiki-workflow-mesh-custom.sh`
- 输出工作区：`/tmp/kyuubiki-custom-mesh-results-20260810`
- 测试节点规模：`elements = 1000 / 10000 / 100000 / 1000000`
- 每组执行链：`validate` → `plan` → `render` → `run --json --dry` → `run --json --execute --executor <executor>`

## 关键配置
- 采用 `direct_mesh_pipeline` workflow 结构后，直接改写 `workflow.steps[0].payload.input.elements`。
- `workflow.id` 也在每组设置为 `custom.direct_mesh_scale_<N>`。

## 结果汇总

| scale | executor | dry.status | dry.steps | exec.status | exec.steps | 注释 |
|---|---|---:|---:|---:|---:|---|
| 1k | mock | ok | 3 | ok | 3 | 通过 |
| 1k | service | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 1k | hybrid | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 10k | mock | ok | 3 | ok | 3 | 通过 |
| 10k | service | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 10k | hybrid | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 100k | mock | ok | 3 | ok | 3 | 通过 |
| 100k | service | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 100k | hybrid | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 1M | mock | ok | 3 | ok | 3 | 通过 |
| 1M | service | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |
| 1M | hybrid | ok | 3 | failed | 0 | `kyuubiki.headless.transport_failure` |

## 发现问题（仅本轮新增）
1. `service` 与 `hybrid` 执行器在第1步 `direct_mesh_solve` 即报连通错误，均为同一错误码与原因：
   - `failed to connect to 127.0.0.1:4000 for service request within 10000 ms: Operation not permitted (os error 1)`
   - 在 `execution_summary.failure.error_code` 中为 `kyuubiki.headless.transport_failure`
   - 代表服务端口不可达或 socket 被环境策略拦截。

2. `hybrid` 对应 service-only workflow 的行为与 `service` 一致（在当前环境均失败），未出现 fallback 到其他执行路径。

3. 自定义工作流（仅改 input）能力确认可用：
   - `headless init` 输出 `template` + `workflow` 结构可直接作为输入。
   - `mock` 路径在 1M 元素规模下仍稳定完成 dry/execute（`executed_step_count=3`）。
   - 这条路径可用于离线验证流程与参数注入逻辑。

4. 服务运行能力在当前环境存在额外限制：尝试启动 web 入口时，日志出现 Mix/PubSub socket 创建失败（`eperm`），提示本地进程网络/IPC 权限不稳定。
   - 运行命令：`node ./scripts/hot-dev.mjs web --help`
   - 触发错误摘要：`failed to open a TCP socket in Mix.Sync.PubSub.subscribe/1, reason: :eperm`
   - 影响：即便服务端启动前置成功，`headless run --execute --executor service` 的基础连通也不稳定。

## 结论
- SDK 的“自定义 workflow 工作流模版”能力是可用的，至少可稳定用于 mock 渗透和规模参数探索。
- 当前环境中服务执行链路主要瓶颈是 transport/connectivity 层（127.0.0.1:4000），不是模板/参数本身。
- 在继续材料研发问题之前，建议先修复服务运行与端口访问链路，再推进更大规模真实仿真。
