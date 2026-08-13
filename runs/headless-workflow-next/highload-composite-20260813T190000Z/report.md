| 回合 | 输入电压(V) | dry-run状态 | execute状态 | winner | score | max_electric_field(V/m) | safety | dry/exe模式 | dry_steps | exec_steps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1400 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 27222.350967729868 | 2204.0712086595927 | dry_run/execute:service | 9 | 9 |
| 2 | 1540 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 29944.592274461174 | 2003.7006832506504 | dry_run/execute:service | 9 | 9 |
| 3 | 1694 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 32939.05976775437 | 1821.545618577033 | dry_run/execute:service | 9 | 9 |
| 4 | 1863 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 36225.19889930671 | 1656.3056055752477 | dry_run/execute:service | 9 | 9 |
| 5 | 2236 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 43478.04999966674 | 1380.006693043039 | dry_run/execute:service | 9 | 9 |
| 6 | 2683 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 52169.82890609052 | 1150.0900282422692 | dry_run/execute:service | 9 | 9 |


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
