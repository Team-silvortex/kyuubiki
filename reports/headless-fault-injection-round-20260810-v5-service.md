# Headless Fault Injection Round v5（service executor 对比）

## 实验范围
- 目的：与 v4（mock-only）同一套用例做 `--executor service` 对照
- 时间：2026-08-10 15:40 (Asia/Shanghai)
- API：`http://127.0.0.1:3000`
- 用例来源：`/tmp/kyuubiki-injection-round-v4`（10 个基础变体，含 duplicate）
- 输出目录：`/tmp/kyuubiki-injection-round-v5-service`

## 关键结果（10 个）

| 用例 | validate_ok | run_status | run_mode | run_executed_step_count | run_error_code | 说明 |
|---|---:|---|---|---:|---|---|
| `risk_invalid` | false | invalid | execute:service | 0 | `kyuubiki.headless.document_validation` | 文档级风险枚举非法，run 直接失败 |
| `index_missing` | false | invalid | execute:service | 0 | `kyuubiki.headless.document_validation` | step 缺 `index`，run 直接失败 |
| `step_timeout_type_string` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 在服务执行阶段尝试提交 step1 失败（连接拒绝） |
| `step_timeout_negative` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 同上 |
| `step_timeout_zero` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 同上 |
| `step_interval_type_string` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 同上 |
| `step_jobwait_bad_ref` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 运行 report 里也保留 `cannot bind future-or-self step 999` |
| `result_fetch_jobid_type_obj` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 同上 |
| `result_fetch_jobid_string` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | 同上 |
| `duplicate_step_index` | false | failed | execute:service | 0 | `kyuubiki.headless.transport_failure` | validation 已报告重复 index 与 step4 payload 问题，但执行仍被 transport 阻断 |

## 与 v4 对比（mock-only）要点

1. v4 的多数 case（`timeout/interval/jobref/result_fetch 类型`）在 mock 下会执行到 `executed_step_count=9` 后返回 `invalid`，可见它会先做较长模拟执行；
   service 下同类用例在 transport 前就未能执行到 step（`executed_step_count=0`），最终主因变成 `kyuubiki.headless.transport_failure`。
2. service 下 run 阶段虽然可见 `run_validation_issue`，但最终被上游 `transport_failure` 遮蔽，导致真实运行路径中“验证问题 + 连通问题”复合，需要前置过滤才能清晰定位。
3. 文档解码级错误（非法 risk、缺 index）在 service 与 mock 行为一致，仍是 `run_status=invalid` 且步数 0。

## 结论

- 本轮不宜直接下“类型约束未生效”结论，因为服务路径整体受控平面连通阻塞覆盖；
- 但它暴露了一个稳定现象：在 service 路径里，`run` 结果的 `run_validation_issue` 和 `error_code=transport_failure` 可能共存，建议调用层按优先级聚合报告（先 contract/validation 再 transport）。

## 附件
- 汇总 JSON（含 10 个用例）：`/tmp/kyuubiki-injection-round-v5-service/fault_injection_round_v5_summary.json`
- 逐用例输出：`/tmp/kyuubiki-injection-round-v5-service/*.out`
- 逐用例报告：`/tmp/kyuubiki-injection-round-v5-service/*-report.json`
