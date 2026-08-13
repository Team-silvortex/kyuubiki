| 回合 | 输入电压(V) | dry-run状态 | execute状态 | winner | score | max_electric_field(V/m) | safety | dry/exe模式 | dry_steps | exec_steps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 900 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 17500.072625912726 | 3428.55719988023 | dry_run/execute:service | 9 | 9 |
| 2 | 990 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 19250.081538088183 | 3116.8699146174567 | dry_run/execute:service | 9 | 9 |
| 3 | 1089 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 21175.091887536637 | 2833.517810390951 | dry_run/execute:service | 9 | 9 |
| 4 | 1198 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 23294.54845469406 | 2575.7099398897967 | dry_run/execute:service | 9 | 9 |
| 5 | 1318 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 25627.89610470152 | 2341.1988153406323 | dry_run/execute:service | 9 | 9 |
| 6 | 1450 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 28194.57981228025 | 2128.06860040052 | dry_run/execute:service | 9 | 9 |
| 7 | 1595 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 31014.04469297819 | 1934.6073881677369 | dry_run/execute:service | 9 | 9 |
| 8 | 1755 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 34125.180649792695 | 1758.2324505691515 | dry_run/execute:service | 9 | 9 |


说明：以下为每轮闭环执行摘要。
关键命令码与状态（0 为成功，1 为失败）：
- headless_init: 0
- headless_validate: 0
- headless_plan: 0
- headless_render: 0
- headless_run_dry_round_1: 0
- headless_run_exec_round_1: 0
- headless_run_bad_alias: 1（预期失败）
- headless_run_missing_report_out: 1（预期失败）

观察结果：
- loop-run-dry.json: mode=dry_run、status=ok、executed_step_count=9、无 block（来自 round-1）。
- loop-run-exec.json: mode=execute:service、status=ok、executed_step_count=9、无 block（来自 round-1）。
- loop-material-report.json: study=composite-thermo-electric-panel，winner 从 round-1 读取
- headless_run_bad_alias 返回 unsupported material report study: study。
- headless_run_missing_report_out 返回 --material-report with --json requires --material-report-out。

结论：
1. 无头闭环在当前版本可稳定打通：init -> validate -> plan -> render -> 多轮 dry-run/execute。
2. 轮间闭环以 winner 的最大场强/得分动态推导下一轮电压，当前路径成功生成每轮报告并形成可追溯链。
3. 已知边界：material-report 的 study 与 bad alias、material-report-out 缺失边界行为继续有效。
