# Headless research: composite thermo-electric panel sweep (2026-08-10)

## Goal
- 用可执行模板 `material_composite_thermo_electric_panel_screening` 跑一轮复合电热-力材料筛选，做参数敏感性对比，检验研发流程和指标响应是否真实。

## Execution command baseline
- `./scripts/kyuubiki headless init --template material_composite_thermo_electric_panel_screening --out tmp/research-round10-multiphysics/base_input.json`
- 批量执行脚本：
  - `./scripts/kyuubiki headless run ... --json --execute --executor service --execution-posture research --allow-sensitive --api-base-url http://127.0.0.1:4000`
- 所有 case 都输出：`run_stdout.json`、`run_stderr.txt`、`run_raw.json`、`material_report.json`。

## Test cases
- `baseline`: 使用基线参数。
- `hot_ambient`: 将所有 `fix_temperature=true` 的热边界温度改为 `85.0`。
- `highfreq`: 将 `electrothermal_loss.frequency_hz` 改为 `20000000.0`。
- `highvoltage`: 将电位边界 `n3`,`n7` 改为 `1200.0V`，并将驱动导体 `n1`,`n5` 电压改为 `0.0000448V`。

## Runtime
- 四组案例均返回 `ok`。
- 每组都是 9 个步骤、3 个求解作业。
- `execution_elapsed_ms` 平均约 `4030–4041ms`，`queue_wait_ms` 均在 `2-3ms`。

## Ranking snapshot（按 rank）

### Baseline
- 1# `copper_ptfe_glass_epoxy` score=0.7300，safety=3428.56，emax=17500.07 V/m，tmax=35.298°C，stress=177,462 Pa
- 2# `aluminum_alumina_aluminum` score=0.5160，safety=5807.25，emax=22385.83 V/m，tmax=35.462°C，stress=274,619 Pa
- 3# `copper_polyimide_aluminum` score=0.3730，safety=15321.96，emax=19579.75 V/m，tmax=36.176°C，stress=699,276 Pa

### Hot ambient
- 1# `copper_ptfe_glass_epoxy` score=0.7300，safety=3423.82，emax=17524.29 V/m，tmax=85.284°C，stress=29,517,078 Pa
- 2# `aluminum_alumina_aluminum` score=0.5030，safety=5805.05，emax=22394.31 V/m，tmax=85.430°C，stress=29,735,504 Pa
- 3# `copper_polyimide_aluminum` score=0.3730，safety=15307.31，emax=19598.49 V/m，tmax=86.305°C，stress=30,264,304 Pa

### High frequency (20 MHz)
- 1# `copper_ptfe_glass_epoxy` score=0.7300，safety=3428.56，emax=17500.08 V/m，tmax=35.328°C，stress=194,883 Pa
- 2# `aluminum_alumina_aluminum` score=0.5630，safety=5807.25，emax=22385.83 V/m，tmax=35.472°C，stress=280,723 Pa
- 3# `copper_polyimide_aluminum` score=0.3730，safety=15321.82，emax=19579.92 V/m，tmax=37.085°C，stress=1,240,197 Pa

### High voltage
- 1# `copper_ptfe_glass_epoxy` score=0.6730，safety=2571.41，emax=23333.51 V/m，tmax=35.530°C，stress=315,319 Pa
- 2# `aluminum_alumina_aluminum` score=0.6000，safety=4355.44，emax=29847.75 V/m，tmax=35.303°C，stress=180,355 Pa
- 3# `copper_polyimide_aluminum` score=0.3730，safety=11491.37，emax=26106.56 V/m，tmax=37.092°C，stress=1,243,905 Pa

## Findings
- 参数修改全部被执行且体现到材料报告中：高温会放大热应力并抬高峰值温度；高电压会显著拉高电场和降低安全系数；高频率改变第二名候选人的评分/应力响应。
- 排名顺序未发生变化（1/2/3 始终固定），但得分/安全裕量对工况敏感，说明当前评分链路已在吞吐层生效。
- 9 步模板的三候选执行链条在该环境稳定跑完，且无失败或重试迹象。
- 环境层面观察到 `Operation not permitted` 端口访问问题：在非提权模式下会导致 `headless run` 无法连接服务；加 `require_escalated` 后可复现成功运行。

## Known risks / action items
- 建议补一个“本地无提权运行”告警或自动降级提示，以减少本地环境里 headless 服务执行的失败定位成本。
- 可进一步加入更高电场工况（例如 1800V 与 2400V）来确认安全因子非线性规律与失效边界。

## Artifacts
- 输入与脚本产物目录：`tmp/research-round10-multiphysics/`
- 结果目录：`tmp/research-round10-multiphysics/cases/{baseline,hot_ambient,highfreq,highvoltage}/`
- 运行日志与报告：对应 `run_stdout.json`、`run_stderr.txt`、`run_raw.json`、`material_report.json`
