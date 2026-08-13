# Kyuubiki 研发轮次（2026-08-13 继续）

- 时间：2026-08-13
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 脚本：`scripts/run_headless_workflow_regression.sh`
- 执行入口：直接调用脚本（`SYNC_SDK_FROM_DEV=1`）

## 1) 多模板 3 轮闭环回归

执行了 6 个场景（3 材料 + 2 direct 电场 + 1 材料）：

1. `material_composite_thermo_electric_panel_screening`
   - 路径：`runs/headless-workflow-next/round-composite-thermo-20260813T180745Z-1`
   - 输入电压：`1000 -> 1100 -> 1210`
   - winner 固定为 `4-layer` 对应材质 `copper_ptfe_glass_epoxy`
   - 状态：所有轮次 `dry=0/exec=0`

2. `material_structural_panel_screening`
   - 路径：`runs/headless-workflow-next/round-structural-plate-20260813T180745Z-2`
   - 输入电压：`1300 -> 1430 -> 1573`
   - winner 固定为 `carbon_fiber_quasi_iso`
   - 状态：所有轮次 `dry=0/exec=0`

3. `material_heat_spreader_screening`
   - 路径：`runs/headless-workflow-next/round-heat-spreader-20260813T180745Z-3`
   - 输入电压：`900 -> 1035 -> 1190`
   - winner 固定为 `pyrolytic_graphite_in_plane`
   - 状态：所有轮次 `dry=0/exec=0`

4. `material_thermo_shield_screening`
   - 路径：`runs/headless-workflow-next/round-thermo-shield-20260813T180745Z-4`
   - 输入电压：`1400 -> 1540 -> 1694`
   - winner 固定为 `invar_36`
   - 状态：所有轮次 `dry=0/exec=0`

5. `direct_electrostatic_triangle`
   - 路径：`runs/headless-workflow-next/round-direct-tri-20260813T180745Z-5`
   - 输入电压：`1000 -> 1050 -> 1102`
   - 无 material-report，`dry=3/exec=3`

6. `direct_electrostatic_quad`
   - 路径：`runs/headless-workflow-next/round-direct-quad-20260813T180745Z-6`
   - 输入电压：`1000 -> 1050 -> 1102`
   - 无 material-report，`dry=3/exec=3`

## 2) 关键闭环行为复核

- `round-direct-tri` / `round-direct-quad` 的 `batch` 已确认边界电位随 round 漂移：
  - triangle: `n0=1000` → `1102`
  - quad: `e0=1000` → `1102`
- 说明 `run_headless_workflow_regression.sh` 对 `solve_electrostatic_plane_triangle_2d` 与 `_plane_quad_2d` 的电位回写已生效。
- 每个 run 都完成了 `headless_run_bad_alias` 与 `headless_run_missing_report_out` 的预期边界行为。

## 3) 当前结论

- 该轮 3-rd closed-loop 在当前代码版本下稳定通过，没有新增阻断性缺陷。
- 电场边界参数在不同求解算子上都可被轮次注入覆盖。

## 2026-08-13 18:16 补充轮次（跨域直接算子 + blocked-study 回归）

### 一、执行参数
- 命令：`scripts/run_headless_workflow_regression.sh`
- 回合数：`HEADLESS_ROUNDS=2`（blocked 模板用 1 轮）
- run 根目录：`/Users/Shared/chroot/dev/kyuubiki/runs/headless-workflow-next`
- 本轮新增模板：
  - `direct_acoustic_bar_1d`
  - `direct_frame_3d`
  - `direct_truss_3d`
  - `direct_beam_1d`
  - `direct_spring_3d`
  - `direct_torsion_1d`
  - `direct_thermal_truss_3d`
  - `direct_heat_triangle`
  - `material_study_envelope_catalog`（blocked-by-design）
  - `material_study_envelope_ranking`（blocked-by-design）

### 二、结果一览
- 1) `round-acoustic-bar-20260813T181603Z-1`
  - 电压：`500 -> 525`
  - dry_steps=3 / exec_steps=3 / 两轮 status 全 0
- 2) `round-direct-frame3d-20260813T181603Z-2`
  - 电压：`700 -> 735`
  - dry_steps=3 / exec_steps=3 / 两轮 status 全 0
- 3) `round-direct-truss3d-20260813T181603Z-3`
  - 电压：`700 -> 735`
  - dry_steps=3 / exec_steps=3 / 两轮 status 全 0
- 4) `round-direct-beam1d-20260813T181603Z-4`
  - 电压：`700 -> 735`
  - 两轮 status 全 0
- 5) `round-direct-spring3d-20260813T181603Z-5`
  - 电压：`700 -> 735`
  - 两轮 status 全 0
- 6) `round-direct-torsion1d-20260813T181603Z-6`
  - 电压：`700 -> 735`
  - 两轮 status 全 0
- 7) `round-direct-thermal-truss3d-20260813T181603Z-7`
  - 电压：`1200 -> 1260`
  - 两轮 status 全 0
- 8) `round-direct-heat-tri-20260813T181603Z-8`
  - 电压：`300 -> 315`
  - 两轮 status 全 0
- 9) `round-envelope-catalog-20260813T181603Z-9`
  - 材料报告类 blocked-by-design，`dry` 被 blocked，`exec=skipped`
- 10) `round-envelope-ranking-20260813T181603Z-10`
  - 与上类似，blocked-by-design，`dry=blocked`，`exec=skipped`

### 三、结论
- 本轮新增跨域直接算子（声学/热/力/扭转）在 2 轮闭环下可稳定跑通。
- `material_study_*_catalog/ranking` 持续维持 blocked-by-design 行为（dry blocked、execute 跳过），边界策略未退化。
- 本轮无新增失败，当前风险主要在于：
  - 该类无 material-report 模板中 winner 仍为 `n/a`，score/safety 字段缺失是设计行为，不代表执行异常；
  - 有效性还需结合真实任务指标（而非闭环的“可重复执行”）做更长时的多轮优化测试。

## 2026-08-13 Option-2 高负载回合（新）

### 1) 多轮闭环高负载回归（`run_headless_workflow_regression.sh`）

执行参数：
- `HEADLESS_ROUNDS=6`
- `HEADLESS_START_VOLTAGE`：`1400 -> 1600`
- `HEADLESS_MAX_VOLTAGE`: `3200 / 4200`
- `API_BASE_URL=http://127.0.0.1:3000`
- `KYUUBIKI_REPO_DIR=/Users/Shared/chroot/dev/kyuubiki`
- `SYNC_SDK_FROM_DEV=0`

记录:
- `runs/headless-workflow-next/highload-composite-20260813T190000Z`
  - rounds: `1400,1540,1694,1863,2236,2683`
  - winner: `copper_ptfe_glass_epoxy`
  - score/场强/safety: `0.73 / 27222.3509677.. -> 52169.8289 / 2204.0712.. -> 1150.0900`
  - all dry/exec status: `ok`
  - dry/exe steps: `9`
- `runs/headless-workflow-next/highload-structural-20260813T190500Z`
  - rounds: `1600,1760,1936,2130,2343,2577`
  - winner: `carbon_fiber_quasi_iso`
  - score/场强/safety: `0.732 / n/a / n/a`
  - all dry/exec status: `ok`
  - dry/exec steps: `9`

结论：材料类模板在 6 轮动态压测下无阻塞退化（`status=0`），`winner` 稳定，`max_electric_field` 单调上升。

### 2) 服务执行模板横向压测（`run_headless_research_matrix.sh --pipeline service`）

输出：`results/headless-research-matrix-20260813T1912-highload-option2`
- 报告：`reports/headless-research-matrix-option2-highload-20260813T1912-20260813-182618.md`

观察：
- direct 类模板 (`direct_mesh_pipeline/direct_thermal_frame_3d/direct_thermal_truss_3d/direct_heat_triangle`) 在服务执行器全部通过，`service_primary_exit=0`，mode=`execute:service`。
- 材料类模板全部通过且无回退触发，winner 与近几轮结果一致：
  - `polyimide_film`（`score=0.8`,`field=30000`,`safety=10000`）
  - `carbon_fiber_quasi_iso`（`score=0.732`）
  - `copper_ptfe_glass_epoxy`（`score=0.73`,`field=17500`,`safety=3428`）
  - `invar_36`（`score=0.75`）
- `service_fallback` 全部为 `skipped`（3000 端口未触发退化）。

### 3) 大网格端口回退压测（`run_service_port_matrix_copy.py`）

命令执行：
- `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki`
- `SERVICE_MATRIX_TS=20260813T191500`
- 入口：`python3 scripts/run_service_port_matrix_copy.sh`

结果：
- 报告：`reports/service-matrix-port-rotation-20260813T191500.md`
- 成功用例：
  - `large_700x700`
  - `large_1000x1000_noids`
  - `small_direct_heat_triangle`
- 以上 3 个用例在 3000 端口均返回：`status=0`、`report.status=ok`、`mode=execute:service`，均未触发 4000 fallback。
- 大网格用例缺失（`input not found`）：
  - `large_3000x1000_noids`
  - `large_2000x1000_noids`
  - `large_2000x2000_noids`
  - `large_2500x1600_noids`
  - `large_1800x2200_noids`
  - `large_3000x900_noids`

### 4) 风险与修复优先级（本轮）

1. 高优先：补齐 dev 的大网格输入集（尤其 1.2M~1.5M 节点规模），避免脚本只能回归部分用例；
2. 中优先：当 `run_service_port_matrix_copy.py` 输出 `input not found` 时，增加“环境清单缺口”提示的归档字段（当前已可读，但缺陷仍影响覆盖率）。

## 2026-08-13 Option-2 压测补齐：端口矩阵 full list（第二轮）

### 操作
- 先补齐 `results/sdk-large-mesh-1m/` 缺失文件（`3000x1000/2000x1000/2000x2000/2500x1600/1800x2200/3000x900`），先行使用 `input_700x700_jobwait_1200000.json` 作为占位，并在 `notes.synthetic_fallback_fixture=true` 标记。
- 重跑端口矩阵脚本：
  - 命令：`python3 scripts/run_service_port_matrix_copy.sh`
  - `SERVICE_MATRIX_TS=20260813T192000`
  - `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki`

### 结果
- 报告：[`service-matrix-port-rotation-20260813T192000.md`]( /Users/Shared/chroot/dev/kyuubiki/reports/service-matrix-port-rotation-20260813T192000.md )
- 9/9 用例均跑通：
  - 3000 端口执行返回 `status=0`、`mode=execute:service`、`service_3000.body-limit-signature=0`
  - 无任何 case 触发 `fallback`
  - `service_4000` 均为 `fallback skipped`
- 本轮可用于验证“矩阵完整执行链”本身不再因 missing fixture 中断。

### 风险说明（重要）
- 本次补齐用例为占位副本，当前仍基于 `input_700x700_jobwait_1200000.json` 的内容，仅用于验证脚本矩阵联通性；
- 若要继续追求“真实 1M 网格边界压力”，还需要在 `results/sdk-large-mesh-1m/` 提供对应分辨率的真实输入文件，再复跑一次以上。该项已列为下一轮任务。 

## 2026-08-13 Option-2 高负载闭环补充（`material_thermo_shield_screening` 8轮）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-thermo-shield-20260813T183344Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_thermo_shield_screening`
- `HEADLESS_ROUNDS=8`
- `START=2000`
- `MAX=5200`
- `MIN=1200`
- `HEADLESS_VOLTAGE_FACTOR=1.1`
- `executor` 仅使用服务执行链（`--executor service`）

### 结果摘要
- `headless_loop` 输出轮次：`2000 -> 2200 -> 2420 -> 2662 -> 2928 -> 3221 -> 3543 -> 3897`
- 每轮 `dry`/`execute` 均成功：`0/0`
- 每轮 `dry_steps=9`，`execute_steps=9`
- `winner` 始终稳定为 `invar_36`，`score` 始终为 `0.75`
- `max_electric_field(V/m)` 在该回合仍为 `n/a`（当前模板与此字段在输出阶段默认缺省）

### 结论
- `material_thermo_shield_screening` 在高负载参数下（8 轮、1.2k~5.2k 电压范围）保持稳定闭环；未发现阻塞、回退或关键字段异常。
- 与此前 3 轮版本相比，参数上界提高后仍未触发新的数值稳定性异常。
- 该场景可列入下一阶段“真实科研任务基线”候选：`invar_36` 路径稳定且闭环可复用。

## 2026-08-13 Option-2 扩展（`direct_mesh_pipeline` 高频闭环）

### 运行路径
- 归档目录：`runs/headless-workflow-next/direct-mesh-highload-20260813T183455Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_mesh_pipeline`
- `HEADLESS_ROUNDS=6`
- `START=1000`
- `MAX=6000`
- `MIN=900`
- `executor`: `--executor service`，仅服务执行

### 结果摘要
- `headless_loop` 输出轮次：`1000 -> 1050 -> 1102 -> 1157 -> 1215 -> 1276`
- 每轮 `dry`/`execute` 均成功：`0/0`
- 每轮 `dry_steps=3`，`execute_steps=3`
- 当前模板无 `material-report`，`winner`、`score`、`max_electric_field`、`safety` 均为 `n/a`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 按预期为 `skipped`

### 结论
- 该路径在高轮次（6 轮）下能保持服务闭环稳定，适合用于非材料直接算子吞吐场景的回归主路径。

## 2026-08-13 Option-2 扩展（`material_dielectric_screening` 高负载 9 轮）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-dielectric-20260813T183711Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_dielectric_screening`
- `HEADLESS_ROUNDS=9`
- `START=900`
- `MAX=6200`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- `headless_loop` 输出轮次：`900 -> 1035 -> 1190 -> 1369 -> 1574 -> 1968 -> 2460 -> 3321 -> 4483`
- `winner` 持续固定为 `polyimide_film`
- `score` 持续为 `0.8`
- `max_electric_field(V/m)` 与 `safety` 单调变化（场强上升、safety下降）
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 本场景在 9 轮高压下闭环稳定，没有出现回退或阻塞。
- `polyimide_film` 路径对 4~4.5kV 电压区间表现连续，适合做后续“真实介电材料”长参数窗口扫描。

## 2026-08-13 Option-2 扩展（`material_heat_spreader_screening` 长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-heat-spreader-20260813T183835Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_heat_spreader_screening`
- `HEADLESS_ROUNDS=8`
- `START=900`
- `MAX=5200`
- `MIN=900`
- `executor`: `--executor service`

### 结果摘要
- `headless_loop` 输出轮次：`900 -> 1035 -> 1190 -> 1369 -> 1574 -> 1810 -> 2082 -> 2394`
- `winner` 持续固定为 `pyrolytic_graphite_in_plane`
- `score` 持续为 `1.0`
- `max_electric_field(V/m)` 与 `safety` 为 `n/a`（当前模板输出中默认缺省）
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 本场景高轮次下表现稳定，闭环不抖动，winner 与 score 完全固定。
- 该场景对“多轮高压下材料参数选择不变性”验证有效，适合作为基线。

## 2026-08-13 Option-2 扩展（`material_dielectric_screening` 高负载 9 轮）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-dielectric-20260813T183711Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_dielectric_screening`
- `HEADLESS_ROUNDS=9`
- `START=900`
- `MAX=6200`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- `headless_loop` 输出轮次：`900 -> 1035 -> 1190 -> 1369 -> 1574 -> 1968 -> 2460 -> 3321 -> 4483`
- `winner` 持续固定为 `polyimide_film`
- `score` 持续为 `0.8`
- `max_electric_field(V/m)` 与 `safety` 单调变化（场强上升、safety下降）
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 本场景在 9 轮高压下闭环稳定，没有出现回退或阻塞。
- `polyimide_film` 路径对 4~4.5kV 电压区间表现连续，适合做后续“真实介电材料”长参数窗口扫描。

## 2026-08-13 Option-2 全模板服务横向回归（最新）

### 运行参数
- 命令：`bash scripts/run_headless_research_matrix.sh --pipeline service`
- `TS=20260813-183849`
- `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki`
- 覆盖模板：`direct_mesh_pipeline/direct_*/direct_heat_triangle/material_* screening`
- 输出：
  - 结果目录：`results/headless-research-matrix-20260813-183849`
  - 报告：`reports/headless-research-matrix-20260813-183849.md`

### 结果要点
- `service_primary_exit=0` 全量通过（除 `material_study_*` 其 `service_primary_status=blocked`，符合设计）
- `service_fallback` 对所有模板均为 `skipped`
- 无论直接求解器还是材料模板，执行器端均为 `http://127.0.0.1:3000`
- 代表性材料模板 winner：
  - `material_composite_thermo_electric_panel_screening` -> `copper_ptfe_glass_epoxy`
  - `material_structural_panel_screening` -> `carbon_fiber_quasi_iso`
  - `material_dielectric_screening` -> `polyimide_film`
  - `material_thermo_shield_screening` -> `invar_36`

### 结论
- 跨模板服务执行的“横向可执行性”在当前版本仍保持稳定。
- 当前主要结构性行为仍是 `direct_*` 无 material-report（winner/n/a）、材料模板 winner 正常。

## 2026-08-13 Option-2 扩展（`material_composite_thermo_electric_panel_screening` 高压极限 10 轮）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-composite-extreme-20260813T184738Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_composite_thermo_electric_panel_screening`
- `HEADLESS_ROUNDS=10`
- `START=1600`
- `MAX=9000`
- `MIN=1200`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`1600 -> 1760 -> 1936 -> 2323 -> 2788 -> 3624 -> 4711 -> 6124 -> 7961 -> 9000`
- `winner` 始终固定为 `copper_ptfe_glass_epoxy`
- `score` 从 `0.73` 微降到 `0.722`
- `max_electric_field(V/m)` 与 `safety` 持续单调趋势（场强上升、safety 下降）
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 在更高电压和更长轮次条件下，闭环仍稳定；未观察到数值异常中断。
- 这说明 `material_composite_thermo_electric_panel_screening` 在参数扩展区间具有较好的执行鲁棒性，但分数出现轻微下滑，提示高场强下优选裕度变化，应继续观察到更高分辨率材料特性边界。

## 2026-08-13 Option-2 扩展（`material_structural_panel_screening` 高压极限 10 轮）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-structural-extreme-20260813T184935Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_structural_panel_screening`
- `HEADLESS_ROUNDS=10`
- `START=1300`
- `MAX=10000`
- `MIN=1200`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`1300 -> 1430 -> 1573 -> 1730 -> 1903 -> 2093 -> 2302 -> 2532 -> 2785 -> 3064`
- `winner` 持续固定为 `carbon_fiber_quasi_iso`
- `score` 持续固定为 `0.732`
- `max_electric_field(V/m)` 与 `safety` 为 `n/a`（当前模板输出设计缺省）
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 高电压长轮次下结构类闭环仍稳定。
- 与 6 轮版本相比，winner 与 score 无漂移，说明该场景在扩展到更高电压下仍偏保守且重复性好。

## 2026-08-13 Option-2 扩展（`material_thermo_shield_screening` 高压扩展 10 轮）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-thermo-shield-extreme-20260813T185111Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_thermo_shield_screening`
- `HEADLESS_ROUNDS=10`
- `START=1100`
- `MAX=10000`
- `MIN=900`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`1100 -> 1210 -> 1331 -> 1464 -> 1610 -> 1771 -> 1948 -> 2143 -> 2357 -> 2593`
- `winner` 始终稳定为 `invar_36`
- `score` 始终 `0.75`
- `max_electric_field(V/m)` 与 `safety` 在该模板当前输出持续为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 热-屏蔽材料在扩展电压窗口下也未触发 winner 或 score 漂移，闭环稳定。
- 该场景可作为“材料工艺候选固定性”基线，下一步可与真实制造约束耦合（如损耗项/厚度耦合）做二次评价。

## 2026-08-13 Option-2 扩展（`direct_mesh_pipeline` 极限长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-direct-mesh-extreme-20260813T185306Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_mesh_pipeline`
- `HEADLESS_ROUNDS=12`
- `START=900`
- `MAX=12000`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`900 -> 945 -> 992 -> 1042 -> 1094 -> 1149 -> 1206 -> 1266 -> 1329 -> 1395 -> 1465 -> 1538`
- 当前模板仍不含 material-report，字段 `winner/score/field/safety` 均为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=3`，`exec_steps=3`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 为 `skipped`（符合无 material-report 设计）

### 结论
- 即使延长至 12 轮，direct 路径依然闭环稳定，适合作为高频率执行器吞吐基线。

## 2026-08-13 Option-2 端口矩阵复测（`run_service_port_matrix_copy.sh`）

### 运行参数
- 命令：`python3 scripts/run_service_port_matrix_copy.sh`
- `TS=20260813T185300`
- `WORKSPACE_DIR=/Users/Shared/chroot/dev/kyuubiki`
- 输出：`reports/service-matrix-port-rotation-20260813T185300.md`

### 结果
- 9/9 用例在 `service_3000` 均通过（`service_3000.shell-status=0`，`status=ok`）
- `service_4000` 全部 `fallback skipped`
- 仍为占位修复的输入文件：`*_jobwait_1200000.json` 作为 3000x/2200x 等大网格占位；请后续替换为真实尺寸输入后重跑，以验证极限网格下是否触发 `frontend_proxy_artifact_limit`
- 本次运行主要确认：端口路由/回退机制在当前样本规模下执行链完整。

## 2026-08-13 Option-2 扩展（`material_dielectric_screening` 极限 12 轮补充）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-dielectric-extreme-20260813T185545Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_dielectric_screening`
- `HEADLESS_ROUNDS=12`
- `START=1000`
- `MAX=14000`
- `MIN=900`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`1000 -> 1150 -> 1323 -> 1521 -> 1901 -> 2376 -> 3208 -> 4331 -> 5847 -> 7893 -> 10656 -> 14000`
- `winner` 持续稳定为 `polyimide_film`
- `score` 持续为 `0.8`
- `max_electric_field(V/m)` 与 `safety` 持续单调（场强上升、safety 降低）
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（均为预期失败）

### 结论
- 在高起点到 14kV 的大范围内，`material_dielectric_screening` 仍保持 winner 与 score 的完全稳定。
- 这是一个可用于“介电材料高压工况”参数延展的强稳态基线。

## 2026-08-13 Option-2 扩展（`material_structural_panel_screening` 长轮次补充）

### 运行路径
- 归档目录：`runs/headless-workflow-next/highload-structural-extreme2-20260813T185743Z`

### 运行参数
- `HEADLESS_TEMPLATE=material_structural_panel_screening`
- `HEADLESS_ROUNDS=12`
- `START=1000`
- `MAX=12000`
- `MIN=900`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`1000 -> 1100 -> 1210 -> 1331 -> 1464 -> 1610 -> 1771 -> 1948 -> 2143 -> 2357 -> 2593 -> 2852`
- `winner` 持续稳定为 `carbon_fiber_quasi_iso`
- `score` 持续稳定为 `0.732`
- `dry/exe` 全部 `0/0`
- `dry_steps=9`，`exec_steps=9`
- 边界回归：`headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）

### 结论
- 结构类在更高起点+更多轮次下仍“winner 与 score 不漂移”，说明闭环控制对该模板较稳健。

## 2026-08-13 Option-2 前者场景（`direct_thermal_truss_3d` 长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/direct-thermal-truss-extreme-20260813T185945Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_thermal_truss_3d`
- `HEADLESS_ROUNDS=12`
- `START=900`
- `MAX=12000`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`900 -> 945 -> 992 -> 1042 -> 1094 -> 1149 -> 1206 -> 1266 -> 1329 -> 1395 -> 1465 -> 1538`
- 当前模板无 material-report，`winner/score/field/safety` 均为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=3`，`exec_steps=3`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 为 `skipped`

### 结论
- 该直接热三维算子在 12 轮高频增长下闭环执行稳定，适合高吞吐基线。

## 2026-08-13 Option-2 前者场景（`direct_torsion_1d` 长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/direct-torsion-extreme-20260813T185949Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_torsion_1d`
- `HEADLESS_ROUNDS=12`
- `START=900`
- `MAX=12000`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`900 -> 945 -> 992 -> 1042 -> 1094 -> 1149 -> 1206 -> 1266 -> 1329 -> 1395 -> 1465 -> 1538`
- 当前模板无 material-report，`winner/score/field/safety` 均为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=3`，`exec_steps=3`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 为 `skipped`

### 结论
- `direct_torsion_1d` 在 12 轮同参数下执行稳定，和 `direct_thermal_truss_3d` 表现一致，均可作为长时直接算子稳定性样本。

## 2026-08-13 Option-2 前者场景（`direct_beam_1d` 长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/direct-beam-extreme-20260813T190217Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_beam_1d`
- `HEADLESS_ROUNDS=12`
- `START=900`
- `MAX=12000`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`900 -> 945 -> 992 -> 1042 -> 1094 -> 1149 -> 1206 -> 1266 -> 1329 -> 1395 -> 1465 -> 1538`
- 当前模板无 material-report，`winner/score/field/safety` 均为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=3`，`exec_steps=3`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 为 `skipped`

### 结论
- `direct_beam_1d` 与前两类直接算子一致，支持 12 轮闭环稳定运行。

## 2026-08-13 Option-2 前者场景（`direct_spring_3d` 长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/direct-spring-extreme-20260813T190454Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_spring_3d`
- `HEADLESS_ROUNDS=12`
- `START=900`
- `MAX=12000`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`900 -> 945 -> 992 -> 1042 -> 1094 -> 1149 -> 1206 -> 1266 -> 1329 -> 1395 -> 1465 -> 1538`
- 当前模板无 material-report，`winner/score/field/safety` 均为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=3`，`exec_steps=3`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 为 `skipped`

### 结论
- `direct_spring_3d` 与其余直接算子一致，支持 12 轮稳定闭环。

## 2026-08-13 Option-2 前者场景（`direct_acoustic_bar_1d` 长轮次）

### 运行路径
- 归档目录：`runs/headless-workflow-next/direct-acoustic-extreme-20260813T190604Z`

### 运行参数
- `HEADLESS_TEMPLATE=direct_acoustic_bar_1d`
- `HEADLESS_ROUNDS=12`
- `START=900`
- `MAX=12000`
- `MIN=800`
- `executor`: `--executor service`

### 结果摘要
- 电压序列：`900 -> 945 -> 992 -> 1042 -> 1094 -> 1149 -> 1206 -> 1266 -> 1329 -> 1395 -> 1465 -> 1538`
- 当前模板无 material-report，`winner/score/field/safety` 均为 `n/a`
- 每轮 `dry`/`execute` 均成功：`0/0`
- `dry_steps=3`，`exec_steps=3`
- `headless_run_bad_alias`、`headless_run_missing_report_out` 为 `skipped`

### 结论
- `direct_acoustic_bar_1d` 12 轮长闭环稳定运行，和 `direct_spring_3d`、`direct_beam_1d`、`direct_torsion_1d`、`direct_thermal_truss_3d` 表现一致。

## 2026-08-13 Option-2 前者场景统一对比汇总（直接算子 5 类长轮次）

| 模板 | 归档目录 | 轮次 | 电压序列(V) | dry/exe 状态 | dry_steps/exec_steps | winner | score | max_electric_field | safety | bad_alias | missing_report_out |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct_thermal_truss_3d | `runs/headless-workflow-next/direct-thermal-truss-extreme-20260813T185945Z` | 12 | `900, 945, 992, 1042, 1094, 1149, 1206, 1266, 1329, 1395, 1465, 1538` | `0 / 0` | `3 / 3` | n/a | n/a | n/a | n/a | skipped | skipped |
| direct_torsion_1d | `runs/headless-workflow-next/direct-torsion-extreme-20260813T185949Z` | 12 | `900, 945, 992, 1042, 1094, 1149, 1206, 1266, 1329, 1395, 1465, 1538` | `0 / 0` | `3 / 3` | n/a | n/a | n/a | n/a | skipped | skipped |
| direct_beam_1d | `runs/headless-workflow-next/direct-beam-extreme-20260813T190217Z` | 12 | `900, 945, 992, 1042, 1094, 1149, 1206, 1266, 1329, 1395, 1465, 1538` | `0 / 0` | `3 / 3` | n/a | n/a | n/a | n/a | skipped | skipped |
| direct_spring_3d | `runs/headless-workflow-next/direct-spring-extreme-20260813T190454Z` | 12 | `900, 945, 992, 1042, 1094, 1149, 1206, 1266, 1329, 1395, 1465, 1538` | `0 / 0` | `3 / 3` | n/a | n/a | n/a | n/a | skipped | skipped |
| direct_acoustic_bar_1d | `runs/headless-workflow-next/direct-acoustic-extreme-20260813T190604Z` | 12 | `900, 945, 992, 1042, 1094, 1149, 1206, 1266, 1329, 1395, 1465, 1538` | `0 / 0` | `3 / 3` | n/a | n/a | n/a | n/a | skipped | skipped |

### 统一结论
- 5 个直接算子在 `--executor service` 与同一电压扩展策略下均表现一致：
  - 无阻塞（`dry=0/0`, `execute=0`）
  - 无 runner 回退（均通过服务主执行器）
  - 无 material-report 解析路径（字段全 `n/a`，边界测试按设计 `skipped`）
- 当前可作为“直接算子长时稳定性”统一基线，用于对照之后引入新算子/新参数的回归偏差。

## 2026-08-13 Option-2 前者场景偏差分析（直接算子长轮次）

### 指标口径
- `dry_status`: 当前以 `0/0` 为绿灯
- `exe_status`: 当前以 `0/0` 为绿灯
- `dry_steps`: 每轮 dry 步数（越接近）  
- `exec_steps`: 每轮 execute 步数（越接近）  
- `winner_parse`: `winner/score/field/safety` 是否可解析且一致  
- `runner_fallback`: 非 service 执行器出现视为异常  

### 一致性对比表

| 模板 | dry_status | exe_status | dry_steps | exec_steps | winner_parse | runner_fallback |
| --- | --- | --- | --- | --- | --- | --- |
| direct_thermal_truss_3d | 0/0 | 0/0 | 3 | 3 | n/a（统一缺失） | 0 |
| direct_torsion_1d | 0/0 | 0/0 | 3 | 3 | n/a（统一缺失） | 0 |
| direct_beam_1d | 0/0 | 0/0 | 3 | 3 | n/a（统一缺失） | 0 |
| direct_spring_3d | 0/0 | 0/0 | 3 | 3 | n/a（统一缺失） | 0 |
| direct_acoustic_bar_1d | 0/0 | 0/0 | 3 | 3 | n/a（统一缺失） | 0 |

### 方差与偏差指标
- `dry_status`：方差 0（全同）
- `exe_status`：方差 0（全同）
- `dry_steps`：方差 0（全同）
- `exec_steps`：方差 0（全同）
- `winner_parse`：一致性为 100%（全部“统一缺失”）
- `runner_fallback`：方差 0（全 0）

### 风险 & 下一步建议
- 当前场景仅体现“稳定性基线”，未观察到模板间行为偏差。
- 下一轮建议放大参数扰动（例如不同 `MAX/MIN/START`、更激进网格/约束设置）以触发第一层偏差点，建立更有区分力的故障特征。

## 2026-08-13 Option-2 前者场景（Material 路径偏移）

### 场景 A：`material_thermo_shield_screening` 长轮次（电压 900→1755）

- 归档目录：`runs/headless-workflow-next/round-material-thermo-shield-extreme-20260813T111057Z`
- 运行参数：`HEADLESS_TEMPLATE=material_thermo_shield_screening`, `HEADLESS_ROUNDS=8`, `HEADLESS_START_VOLTAGE=900`, `HEADLESS_MAX_VOLTAGE=12000`, `HEADLESS_MIN_VOLTAGE=800`, `--executor service`
- 轮次摘要：`900 -> 990 -> 1089 -> 1198 -> 1318 -> 1450 -> 1595 -> 1755`
- winner：`invar_36`（全轮）
- score：`0.75`（全轮）
- field/safety：`n/a`（本场景未产出场强数值字段）
- 每轮 `dry/run`：`0/0`, `dry_steps=9`, `exec_steps=9`
- `headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）

### 场景 B：`material_structural_panel_screening` 长轮次（电压 900→1755）

- 归档目录：`runs/headless-workflow-next/round-material-structural-extreme-20260813T111228Z`
- 运行参数：`HEADLESS_TEMPLATE=material_structural_panel_screening`, `HEADLESS_ROUNDS=8`, `HEADLESS_START_VOLTAGE=900`, `HEADLESS_MAX_VOLTAGE=12000`, `HEADLESS_MIN_VOLTAGE=800`, `--executor service`
- 轮次摘要：`900 -> 990 -> 1089 -> 1198 -> 1318 -> 1450 -> 1595 -> 1755`
- winner：`carbon_fiber_quasi_iso`（全轮）
- score：`0.732`（全轮）
- field/safety：`n/a`（本场景未产出场强/安全因子）
- 每轮 `dry/run`：`0/0`, `dry_steps=9`, `exec_steps=9`
- `headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）

### 场景 C：`material_dielectric_screening` 长轮次（电压加速增长）

- 归档目录：`runs/headless-workflow-next/round-material-dielectric-extreme-20260813T111338Z`
- 运行参数：`HEADLESS_TEMPLATE=material_dielectric_screening`, `HEADLESS_ROUNDS=8`, `HEADLESS_START_VOLTAGE=900`, `HEADLESS_MAX_VOLTAGE=12000`, `HEADLESS_MIN_VOLTAGE=800`, `--executor service`
- 轮次摘要：`900 -> 1035 -> 1190 -> 1369 -> 1574 -> 1968 -> 2460 -> 3321`
- winner：`polyimide_film`（全轮）
- score：`0.8`（全轮）
- field：`22500 -> 25875 -> 29750 -> 34225 -> 39350 -> 49200 -> 61500 -> 83025`
- safety：`13333.333... -> 11594.202... -> 10084.033... -> 8765.522... -> 7623.888... -> 6097.560... -> 4878.048... -> 3613.369...`
- 每轮 `dry/run`：`0/0`, `dry_steps=9`, `exec_steps=9`
- `headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）

### Material 场景偏差分析

#### 指标口径
- `电压放大率`: 当前一轮到下一轮的倍率
- `winner稳定性`: winner 是否跨轮一致
- `score稳定性`: score 是否跨轮一致
- `material_report可解析性`: winner/score/field/safety 的可读性
- `dry/exec链路`: `status` 与 `executed_step_count`

#### 对比表

| 场景 | 电压放大率行为 | winner稳定性 | score稳定性 | field解析 | safety解析 | dry/exec |
| --- | --- | --- | --- | --- | --- | --- |
| material_thermo_shield_screening | 1.10,1.10,1.10,1.10...（线性增长） | 稳定（全轮 invar_36） | 稳定（0.75） | n/a | n/a | `0/9` & `0/9` |
| material_structural_panel_screening | 1.10,1.10,1.10,1.10...（线性增长） | 稳定（全轮 carbon_fiber_quasi_iso） | 稳定（0.732） | n/a | n/a | `0/9` & `0/9` |
| material_dielectric_screening | 1.15,1.16,1.15,1.15,1.25,1.25,1.35（先加速后更强） | 稳定（全轮 polyimide_film） | 稳定（0.8） | 有解析值（持续递增） | 有解析值（持续递减） | `0/9` & `0/9` |

#### 关键结论
- 这是首次出现“同轮数策略下模板间行为分歧”的信号：三组场景中，电压更新策略对 `material_dielectric_screening` 触发 `field>50000` 条件，出现了倍率变化和更强非线性增长。
- 在 `material-dielectric` 中出现了可解释的物理指标链路（`field` 上升、`safety` 下降），可作为后续“可触发边界（失稳/击穿）”预警阈值的初始曲线。
- 三组场景均通过 `execute:service` 且 dry/exec step 恒定，当前看属于“稳定执行+策略差异敏感”而非稳定性回归缺陷。

### 场景 D：`material_composite_thermo_electric_panel_screening` 长轮次（电压 900→1755）

- 归档目录：`runs/headless-workflow-next/round-material-composite-thermo-extreme-20260813T111826Z`
- 运行参数：`HEADLESS_TEMPLATE=material_composite_thermo_electric_panel_screening`, `HEADLESS_ROUNDS=8`, `HEADLESS_START_VOLTAGE=900`, `HEADLESS_MAX_VOLTAGE=12000`, `HEADLESS_MIN_VOLTAGE=800`, `--executor service`
- 轮次摘要：`900 -> 990 -> 1089 -> 1198 -> 1318 -> 1450 -> 1595 -> 1755`
- winner：`copper_ptfe_glass_epoxy`（全轮）
- score：`0.7300000000000001`（全轮）
- field：`17500.07 -> 19250.08 -> 21175.09 -> 23294.55 -> 25627.90 -> 28194.58 -> 31014.04 -> 34125.18`
- safety：`3428.56 -> 3116.87 -> 2833.52 -> 2575.71 -> 2341.20 -> 2128.07 -> 1934.61 -> 1758.23`
- 每轮 `dry/run`：`0/0`, `dry_steps=9`, `exec_steps=9`
- `headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）

### 场景 E：`material_heat_spreader_screening` 长轮次（电压 900→2394）

- 归档目录：`runs/headless-workflow-next/round-material-heat-spreader-extreme-20260813T111939Z`
- 运行参数：`HEADLESS_TEMPLATE=material_heat_spreader_screening`, `HEADLESS_ROUNDS=8`, `HEADLESS_START_VOLTAGE=900`, `HEADLESS_MAX_VOLTAGE=12000`, `HEADLESS_MIN_VOLTAGE=800`, `--executor service`
- 轮次摘要：`900 -> 1035 -> 1190 -> 1369 -> 1574 -> 1810 -> 2082 -> 2394`
- winner：`pyrolytic_graphite_in_plane`（全轮）
- score：`1.0`（全轮）
- field/safety：`n/a`（本场景未产出字段）
- 每轮 `dry/run`：`0/0`, `dry_steps=9`, `exec_steps=9`
- `headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）

### 场景对比更新（D/E）

- `material_composite_thermo_electric_panel_screening`：出现 `field` 与 `safety` 可解析链路，并且 field 单调增长，safety 逐轮下降；说明当前闭环下模型对场强响应敏感，适合后续引入安全阈值报警。
- `material_heat_spreader_screening`：全程 winner/score 恒定、`field/safety` 缺省，主要作用在于稳定性和边界回归验证。
- 建议下一步：对 `material_composite_thermo_electric_panel_screening` 逐步下放 `HEADLESS_MAX_VOLTAGE` 至 3000~9000 小步段，观察 score 是否从 `0.73` 出现可观测拐点。

### 场景 F：`material_composite_thermo_electric_panel_screening` 高压段 12 轮（`START=1800`, `MAX=9000`）

- 归档目录：`runs/headless-workflow-next/round-material-composite-thermo-extreme-knee-20260813T112434Z`
- 运行参数：`HEADLESS_TEMPLATE=material_composite_thermo_electric_panel_screening`, `HEADLESS_ROUNDS=12`, `HEADLESS_START_VOLTAGE=1800`, `HEADLESS_MAX_VOLTAGE=9000`, `HEADLESS_MIN_VOLTAGE=900`, `--executor service`
- 轮次摘要：`1800 -> 2160 -> 2592 -> 3370 -> 4381 -> 5695 -> 7404 -> 9000 -> 9000 -> 9000 -> 9000 -> 9000 -> 9000`
- winner：`copper_ptfe_glass_epoxy`（全轮）
- score：`0.7300000000000001 -> 0.7300000000000001 -> 0.7300000000000001 -> 0.7270000000000001 -> 0.7240000000000001 -> 0.7230000000000001 -> 0.7220000000000001 -> 0.7220000000000001 -> ...`
- field：`35000.19 -> 42000.26 -> 50400.36 -> 65528.40 -> 85187.25 -> 110738.34 -> 143971.19 -> 175007.83 -> 175007.83（后续不变）`
- safety：`1714.28 -> 1428.56 -> 1190.47 -> 915.63 -> 704.33 -> 541.82 -> 416.75 -> 342.84 -> 342.84（后续不变）`
- 每轮 `dry/run`：`0/0`, `dry_steps=9`, `exec_steps=9`
- `headless_run_bad_alias=1`、`headless_run_missing_report_out=1`（预期）
- 关键发现：当 voltage 从 2592 上冲到 3370（field 已超过约 5e4）后，`score` 出现明显阶梯式下滑（`-0.003`、再到 `-0.001`、`-0.001`），且 safety 进入快速衰减区；这说明该模板在高场强段开始进入“稳定性下降区”。

### 脚本与流程偏差（待修）

- `scripts/run_headless_workflow_regression.sh` 有一次关键行为差异：脚本内部先前置 `WORKSPACE_DIR="$(cd ...)"` 固定到仓库根，导致外部传入的 `WORKSPACE_DIR` 无效，所有场景都会写到 `/Users/Shared/chroot/dev/kyuubiki/headless-loop` 后再手工拷贝归档。
- 风险：长时间批量跑时，`headless-loop` 文件会被覆盖，容易出现跨场景污染，且报告中的归档路径不可追踪。
- 建议修复：对 `WORKSPACE_DIR` 做“环境变量优先”处理（如 `WORKSPACE_DIR="${WORKSPACE_DIR:-...}"`），并在完成后按 `RUN_ID` 自动持久化 run artifact。
