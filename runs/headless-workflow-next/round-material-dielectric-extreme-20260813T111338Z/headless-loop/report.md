| 回合 | 输入电压(V) | dry-run状态 | execute状态 | winner | score | max_electric_field(V/m) | safety | dry/exe模式 | dry_steps | exec_steps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 900 | 0 | 0 | polyimide_film | 0.8 | 22500.0 | 13333.333333333334 | dry_run/execute:service | 9 | 9 |
| 2 | 1035 | 0 | 0 | polyimide_film | 0.8 | 25874.999999999996 | 11594.202898550726 | dry_run/execute:service | 9 | 9 |
| 3 | 1190 | 0 | 0 | polyimide_film | 0.8 | 29750.0 | 10084.033613445377 | dry_run/execute:service | 9 | 9 |
| 4 | 1369 | 0 | 0 | polyimide_film | 0.8 | 34225.0 | 8765.522279035793 | dry_run/execute:service | 9 | 9 |
| 5 | 1574 | 0 | 0 | polyimide_film | 0.8 | 39350.0 | 7623.888182973316 | dry_run/execute:service | 9 | 9 |
| 6 | 1968 | 0 | 0 | polyimide_film | 0.8 | 49200.0 | 6097.5609756097565 | dry_run/execute:service | 9 | 9 |
| 7 | 2460 | 0 | 0 | polyimide_film | 0.8 | 61500.0 | 4878.048780487805 | dry_run/execute:service | 9 | 9 |
| 8 | 3321 | 0 | 0 | polyimide_film | 0.8 | 83025.0 | 3613.369467028004 | dry_run/execute:service | 9 | 9 |


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
