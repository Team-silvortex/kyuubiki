# Headless Fault-Injection Round (2026-08-10)

## 环境与范围
- 工作目录：`/tmp/kyuubiki-injection-round-v2`
- 执行模式：`execute:mock`
- 覆盖 case：共 10 个（前 5 个为主故障注入 + 前一轮补充 5 个）
- 本轮目标：验证参数/语义/引用容错在 headless mock 执行链路中的可观测性与一致性

## 用例结果汇总

| 用例 | validate_ok | validate_issue_count | run_status | run_exit | run_executed_step_count | run_issues |
|---|---:|---:|---|---:|---:|---|
| `fault_missing_payload_mid` | false | 1 | invalid | n/a | 9 | missing required payload key `model` |
| `fault_bad_payload_type_mid` | false | 1 | invalid | n/a | 9 | missing required payload key `model`（同上） |
| `fault_bad_timeout_type_mid` | true | 0 | ok | 0 | 9 | 无 |
| `fault_bad_job_ref_mid` | true | 0 | ok | 0 | 9 | 无 |
| `fault_bad_action_mid` | false | 3 | invalid | n/a | 9 | unsupported action `solve_non_existing` |
| `fault_result_fetch_missing_job` | false | 1 | invalid | 1 | 3 | missing required payload key `job_id` |
| `fault_result_fetch_bad_payload_type` | true | 0 | ok | 0 | 2 | 无 |
| `fault_negative_timeout` | true | 0 | ok | 0 | 3 | 无 |
| `fault_zero_timeout` | true | 0 | ok | 0 | 3 | 无 |
| `fault_missing_result_fetch_step` | true | 0 | ok | 0 | 2 | 无 |

## 关键发现

### P0/P1（高优先级）
1. **结构校验未阻断部分执行链路仍然跑完**：`run_status=invalid` 的 case（如 missing payload、bad action）仍出现 `run_executed_step_count=9`。这会误导为“部分可执行”；建议一旦 `validate_ok=false`，在执行器侧直接短路并返回明确中断原因。

2. **关键字段缺失仍通过执行入口（仅部分被发现）**：`missing_payload_mid` 与 `bad_payload_type_mid` 在校验都失败，`step 4 (solve_heat_plane_triangle_2d) is missing required payload key model`，说明该动作对 `model` 这类关键输入较敏感，mock 与真实路径需对齐。

### P2（中优先级）
3. **mock 执行路径对类型语义过于宽容**：
   - `fault_bad_timeout_type_mid`（如把 `timeout_ms` 写为字符串）在 validate 与 run 均返回通过。
   - `fault_bad_job_ref_mid`（指向不存在 job）未被执行期拒绝。
   - `fault_result_fetch_bad_payload_type`、`negative/zero timeout`、`missing result_fetch_step` 均为 `ok`。
   建议至少对关键执行元数据做显式约束（类型、范围、引用存在性）以防把“非法工作流”假阳性。

4. **结果获取节点的严格性不一致**：
   - `fault_result_fetch_missing_job` 被正确拦截（`invalid` + 缺少 `job_id`）。
   - 同时 `result_fetch` 的其他语义错误却可能未被发现（bad payload type 为 ok），与上面一致，说明校验规则未覆盖全。

## 建议修复条目（可落单）

1. 统一执行入口与校验入口的语义约束：新增字段类型和引用完整性校验（如 `timeout_ms: int > 0`、`job_id` 存在且已生成）。
2. 对于 `validate_ok=false` 强制 `run_status=invalid` 且 `run_executed_step_count=0`（或明确停止在失败步骤）。
3. 增补 mock 级别的规则测试：
   - `result_fetch` payload 字段类型（`job_id`、`format`、`storage`）
   - 非法超时（0/负数/非数值）
   - 不存在 step 引用
4. 与真实执行器保持一致（尤其是关键算子 `solve_*` 的必需 payload key），避免 mock 与生产判定差异。

## 附件文件
- 汇总：`/tmp/kyuubiki-injection-round-v2/fault_injection_round_v2_summary.json`
- 补充汇总：`/tmp/kyuubiki-injection-round-v2/fault_injection_round_v2_summary_extra.json`
- 详细输入/输出：见 `/tmp/kyuubiki-injection-round-v2/*`
