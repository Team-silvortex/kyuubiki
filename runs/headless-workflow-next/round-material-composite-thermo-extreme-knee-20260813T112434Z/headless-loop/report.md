| 回合 | 输入电压(V) | dry-run状态 | execute状态 | winner | score | max_electric_field(V/m) | safety | dry/exe模式 | dry_steps | exec_steps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1800 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 35000.188103223416 | 1714.2765011161232 | dry_run/execute:service | 9 | 9 |
| 2 | 2160 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 42000.25589732625 | 1428.562724633771 | dry_run/execute:service | 9 | 9 |
| 3 | 2592 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7300000000000001 | 50400.35922600139 | 1190.4677054175872 | dry_run/execute:service | 9 | 9 |
| 4 | 3370 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7270000000000001 | 65528.398078921855 | 915.6335536805966 | dry_run/execute:service | 9 | 9 |
| 5 | 4381 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7240000000000001 | 85187.25430090351 | 704.3307181619496 | dry_run/execute:service | 9 | 9 |
| 6 | 5695 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7230000000000001 | 110738.33771268456 | 541.8177772875065 | dry_run/execute:service | 9 | 9 |
| 7 | 7404 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7220000000000001 | 143971.19233933362 | 416.75003884515104 | dry_run/execute:service | 9 | 9 |
| 8 | 9000 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7220000000000001 | 175007.82564798597 | 342.8418116609547 | dry_run/execute:service | 9 | 9 |
| 9 | 9000 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7220000000000001 | 175007.82564798597 | 342.8418116609547 | dry_run/execute:service | 9 | 9 |
| 10 | 9000 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7220000000000001 | 175007.82564798597 | 342.8418116609547 | dry_run/execute:service | 9 | 9 |
| 11 | 9000 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7220000000000001 | 175007.82564798597 | 342.8418116609547 | dry_run/execute:service | 9 | 9 |
| 12 | 9000 | 0 | 0 | copper_ptfe_glass_epoxy | 0.7220000000000001 | 175007.82564798597 | 342.8418116609547 | dry_run/execute:service | 9 | 9 |


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
