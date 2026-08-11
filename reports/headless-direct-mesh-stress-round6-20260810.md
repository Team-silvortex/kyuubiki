# Headless Direct Mesh Stress（2026-08-10，Round 6）

## 目标
在 `direct_mesh_pipeline` 上做规模拉升实验（`elements` 字段），确认执行器行为是否稳定，重点看 mock 在大规模输入下是否继续 pass，以及 service/hybrid 的真实连通失败是否与规模无关。

## 覆盖参数
- 模板：`template.direct_mesh_pipeline`
- 规模（elements）：`4`、`1000`、`100000`、`1000000`
- 执行器：`mock`、`service`、`hybrid`

## 实验设置
- 路径：`/tmp/kyuubiki-template-matrix-round6-20260810`
- 步骤：`init -> validate -> plan -> render -> run --dry -> run --execute --executor <mock|service|hybrid> --allow-sensitive`

## 结果汇总
- 总记录：12（4 规模 × 3 执行器）
- `mock`：4/4 通过
- `service`：0/4 通过
- `hybrid`：0/4 通过

### 规模对比（按 scale）
| elements | mock | service | hybrid |
| --- | --- | --- | --- |
| 4 | pass | transport_failure | transport_failure |
| 1000 | pass | transport_failure | transport_failure |
| 100000 | pass | transport_failure | transport_failure |
| 1000000 | pass | transport_failure | transport_failure |

## 关键现象
1. mock 对 1,000,000 元件的 `direct_mesh_pipeline` 仍可执行成功（`status=ok`，`executed_step_count=3`）。
2. 1000、100000、1000000 三个级别下，`service` 与 `hybrid` 都在 `direct_mesh_solve` 第一层失败，统一错误：
   - `kyuubiki.headless.transport_failure`
   - `failed to connect to 127.0.0.1:4000 for service request within 10000 ms: Operation not permitted (os error 1)`
3. 失败与 scale 无关，说明此次实验进一步证明了服务端连通性问题是主因，而非输入规模。

## 证据链接
- 结果总表：[/tmp/kyuubiki-template-matrix-round6-20260810/summary.json](/tmp/kyuubiki-template-matrix-round6-20260810/summary.json)
- 聚合表：[/tmp/kyuubiki-template-matrix-round6-20260810/aggregate.json](/tmp/kyuubiki-template-matrix-round6-20260810/aggregate.json)
- 对比表：[/tmp/kyuubiki-template-matrix-round6-20260810/compare-table.md](/tmp/kyuubiki-template-matrix-round6-20260810/compare-table.md)

## 下一步建议
- 先把 service 通道恢复，再复测 `direct_mesh_pipeline` 的更大规模样例（如 5M、1M+）观察服务端算力与任务编排稳定性。
- 下一轮可继续覆盖 `materials` 的候选规模扩展（候选条目、节点/单元数组长度扩展）来补齐“数据规模”缺口。
