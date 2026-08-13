| 回合 | 输入电压(V) | dry-run状态 | execute状态 | winner | score | max_electric_field(V/m) | safety | dry/exe模式 | dry_steps | exec_steps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1000 | 0 | 0 | polyimide_film | 0.8 | 25000.0 | 12000.0 | dry_run/execute:service | 9 | 9 |
| 2 | 1150 | 0 | 0 | polyimide_film | 0.8 | 28750.0 | 10434.782608695652 | dry_run/execute:service | 9 | 9 |
| 3 | 1323 | 0 | 0 | polyimide_film | 0.8 | 33075.0 | 9070.2947845805 | dry_run/execute:service | 9 | 9 |
| 4 | 1521 | 0 | 0 | polyimide_film | 0.8 | 38025.0 | 7889.546351084813 | dry_run/execute:service | 9 | 9 |
| 5 | 1901 | 0 | 0 | polyimide_film | 0.8 | 47525.0 | 6312.46712256707 | dry_run/execute:service | 9 | 9 |
| 6 | 2376 | 0 | 0 | polyimide_film | 0.8 | 59400.0 | 5050.50505050505 | dry_run/execute:service | 9 | 9 |
| 7 | 3208 | 0 | 0 | polyimide_film | 0.8 | 80199.99999999999 | 3740.64837905237 | dry_run/execute:service | 9 | 9 |
| 8 | 4331 | 0 | 0 | polyimide_film | 0.8 | 108275.0 | 2770.722696836758 | dry_run/execute:service | 9 | 9 |
| 9 | 5847 | 0 | 0 | polyimide_film | 0.8 | 146175.0 | 2052.334530528476 | dry_run/execute:service | 9 | 9 |
| 10 | 7893 | 0 | 0 | polyimide_film | 0.8 | 197325.0 | 1520.3344735841886 | dry_run/execute:service | 9 | 9 |
| 11 | 10656 | 0 | 0 | polyimide_film | 0.8 | 266400.0 | 1126.126126126126 | dry_run/execute:service | 9 | 9 |
| 12 | 14000 | 0 | 0 | polyimide_film | 0.8 | 350000.0 | 857.1428571428571 | dry_run/execute:service | 9 | 9 |


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
- loop-material-report.json: study=dielectric-screening，winner 从 round-1 读取
- headless_run_bad_alias 返回 unsupported material report study: study。
- headless_run_missing_report_out 返回 --material-report with --json requires --material-report-out。

结论：
1. 无头闭环在当前版本可稳定打通：init -> validate -> plan -> render -> 多轮 dry-run/execute。
2. 轮间闭环以 winner 的最大场强/得分动态推导下一轮电压，当前路径成功生成每轮报告并形成可追溯链。
3. 已知边界：material-report 的 study 与 bad alias、material-report-out 缺失边界行为继续有效。
