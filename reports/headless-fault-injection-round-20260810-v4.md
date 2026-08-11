# Headless Fault Injection Round v4 (mock-only, 2026-08-10)

## 实验入口
- 时间：2026-08-10 15:39 (Asia/Shanghai)
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 脚本目录：`/tmp/kyuubiki-injection-round-v4`
- 执行策略：每个用例都跑 `headless validate` 与 `headless run --execute --executor mock`

## 用例结论（9 个）

| 用例 | validate_ok | validate_issue_count | validate_schema_error | run_status | run_mode | run_executed_step_count | run_validation_issue_count | 备注 |
|---|---:|---:|---|---|---|---:|---:|---|
| `risk_invalid` | false | 0 | document_validation | invalid | execute:mock | 0 | 1 | 风险值非法（`criticalish`）被 doc-level 捕获，未执行任一步 |
| `index_missing` | false | 0 | document_validation | invalid | execute:mock | 0 | 1 | 缺 `index` 在 step2，未执行任一步 |
| `step_timeout_type_string` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 1 | job_wait timeout 为字符串，`validate` 通过但 run 在后置阶段失败，执行到 step4 后失败 |
| `step_timeout_negative` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 1 | timeout=-1 同上 |
| `step_timeout_zero` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 1 | timeout=0 时继续执行，直到 step4 缺 payload 模型报错 |
| `step_interval_type_string` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 1 | interval_ms 为字符串，仍按旧路径执行并后置失败 |
| `step_jobwait_bad_ref` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 2 | job_wait 未来/不存在引用（999）被 run 阶段识别，但仍执行大量步骤后再报 step4 payload 错误 |
| `result_fetch_jobid_type_obj` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 1 | result_fetch.job_id 为对象没被早期拦截 |
| `result_fetch_jobid_string` | false | 0 | headless_command_failed | invalid | execute:mock | 9 | 1 | result_fetch.job_id 为数字没被早期拦截 |
| `duplicate_step_index` | false | 0 | headless_command_failed | invalid | execute:mock | 10 | 2 | 生成重复 index，既报 step4 payload 缺失，也报 `step 10 index should be 10, received 2` |

> 注意：`validate` 输出在多数组合故障下出现 `document_validation`/`headless_command_failed` 错误封装，`issue_count` 在这些路径上不总是落到 schema 顶层字段。

## 关键新增发现

1. **`run` 的 validate 失败短路仍不可靠**：
   - 多个 case 在 `validate` 阶段未返回可读 `issue`，`run` 会继续执行直到较早的错误（通常 `step4 missing required payload key model`）。
   - 这会导致错误定位偏移，且 `run` 已经做了大量模拟执行（`executed_step_count=9`）。
2. **类型语义在 mock 侧仍不一致**：
   - 对 `job_wait.timeout_ms` / `interval_ms` / `result_fetch.job_id` 的类型错误仍未在 front validate 拦截。
   - 仍以“后置失败”表现，建议在文档 decode 或预检阶段统一类型约束。
3. **引用语义有部分实现**：
   - 对 `job_wait` 自未来引用有提示（如 step2->999），说明引用解析存在，但并未阻断到最早失败点。

## 与上轮交接（v2）的衔接

- v2 已经覆盖 `--executor mock` 下 `timeout_ms` 字符串、`job_id` 不存在引用等 case，依然为 ok；本轮确认该类问题持续存在。

## 附件文件
- 逐例输出目录：`/tmp/kyuubiki-injection-round-v4/`
- 汇总 JSON：`/tmp/kyuubiki-injection-round-v4/fault_injection_round_v4_summary.json`
