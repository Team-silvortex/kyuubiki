# 2026-08-10 21:10 材料研发向模板矩阵（第2轮）

## 目标
- 聚焦材料研发导向模板（`material_*`）做 headless 全链路回归。
- 覆盖 `init -> validate -> plan -> run(--json)`，并对可执行部分再测 `--execute --executor mock`。
- 特别验证 `material_study_*` 的敏感确认链路。

## 证据路径
- 执行目录：`/private/tmp/kyuubiki_mat_round`
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`

## 1) 模板分类与检索
- `headless templates --category materials --json` -> `template_count = 7`
- `headless templates --query screening --json` -> `template_count = 5`
- `headless templates --tag dielectric --json` -> `template_count = 1`

### materials 模板清单
- `material_heat_spreader_screening`
- `material_dielectric_screening`
- `material_thermo_shield_screening`
- `material_structural_panel_screening`
- `material_composite_thermo_electric_panel_screening`
- `material_study_envelope_ranking`
- `material_study_envelope_catalog`

## 2) 预演矩阵（dry-run）
以下模板执行 `init -> validate -> plan -> run --json --report-out`（`run` 不加 `--execute`）

- `material_heat_spreader_screening`：`run status=ok` `mode=dry_run` `executed_step_count=9`
- `material_dielectric_screening`：`run status=ok` `mode=dry_run` `executed_step_count=9`
- `material_thermo_shield_screening`：`run status=ok` `mode=dry_run` `executed_step_count=9`
- `material_structural_panel_screening`：`run status=ok` `mode=dry_run` `executed_step_count=9`
- `material_composite_thermo_electric_panel_screening`：`run status=ok` `mode=dry_run` `executed_step_count=9`
- `material_study_envelope_ranking`：`run status=blocked` `blocked_by_confirmation={'index': 1, 'risk': 'sensitive'}`
- `material_study_envelope_catalog`：`run status=blocked` `blocked_by_confirmation={'index': 1, 'risk': 'sensitive'}`

备注：两类 `material_study_*` 在 step 1（提交图/目录工作流）即触发 sensitive confirmation。

## 3) execute:mock 覆盖
- `material_heat_spreader_screening`：`--execute --executor mock --json` -> `ok`, `mode=execute:mock`, `executed_step_count=9`
- `material_dielectric_screening`：`ok`, `mode=execute:mock`, `executed_step_count=9`
- `material_thermo_shield_screening`：`ok`, `mode=execute:mock`, `executed_step_count=9`
- `material_structural_panel_screening`：`ok`, `mode=execute:mock`, `executed_step_count=9`
- `material_composite_thermo_electric_panel_screening`：`ok`, `mode=execute:mock`, `executed_step_count=9`

## 4) 研究姿态 / 安全 flag 实验（新发现）
### 4.1 posture 与 executor 冲突
- `--execution-posture research --executor mock`：返回 `headless_command_failed`
  - `research execution requires --executor service; mock cannot provide a no-mock execution guarantee`

### 4.2  `material_study_*` 的敏感确认绕过
- 对 `material_study_envelope_ranking` 加 `--allow-sensitive` 后，dry-run 与 execute 都可跑通：
  - `headless run ... --allow-sensitive --json` -> `status=ok`, `mode=dry_run`, `executed=3`
  - `headless run ... --allow-sensitive --execute --executor mock --execution-posture preview --json` -> `status=ok`, `mode=execute:mock`, `executed=3`
- 结论：敏感确认逻辑可以通过 `--allow-sensitive` 预先授予，而默认会 block。

## 5) 结果解读
- 本轮材料模板在 mock 与 dry-run 路径稳定。
- `material_study_*` 的两处行为明确：默认敏感阻断；开启 `--allow-sensitive` 后可继续。
- 从命令层面看，`mock` 对应的执行后端对材料复合/筛选流程兼容度高（9 步模板都能完整执行）。

## 建议（可直接交由修复）
1. 可将 `--allow-sensitive` 在 dry-run 与 execute 两种路径的行为文案在帮助/报错里统一展示“确认字段位置（index/risk）+ 开关名称”。
2. 对 `material_study_*` 增加一条“默认会触发敏感确认”的显式 precheck 文案，避免用户盲猜。
