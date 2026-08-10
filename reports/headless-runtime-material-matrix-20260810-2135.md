# 2026-08-10 21:35 Runtime风格一致性 + 多场景混合矩阵

## 1) 本轮目标
- 复测 `runtime-style` 与 `template` 的配套行为（是否清晰、是否可诊断）。
- 做一轮更杂的多场景模板矩阵（含电磁/热/力/网格/材料），验证 `dry_run + execute:mock` 稳定性。

## 2) 环境与证据
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 临时工作区：`/private/tmp/kyuubiki_runtime_round`

## 3) runtime-style 与模板索引
- `headless templates --runtime browser_only --json` => `template_count = 1`
  - `browser_capture_review`
- `headless templates --runtime service_only --json` => `template_count = 35`
- `headless templates --runtime hybrid --json` => `template_count = 1`
  - `browser_submit_then_poll`

### 查询交叉校验
- `--runtime browser_only --category materials --json` => `template_count = 0`
- `--runtime browser_only --category acoustic --json` => `template_count = 0`

## 4) mismatch/边界测试
以下均通过 shell 捕获返回日志：

1. `--runtime-style browser_only --template direct_truss_3d`
   - 返回：`headless_command_failed` + `unknown headless template "direct_truss_3d"`
   - 但实际 `direct_truss_3d` 在 `--runtime service_only` 集合内存在。
   - 建议：错误应更明确为“该模板与所选 runtime_style 不兼容”。

2. `--runtime-style service_only --template browser_capture_review`
   - 结果同上：`unknown headless template "browser_capture_review"`
   - 实际该模板在 `browser_only` 集合内存在。

3. `--runtime-style service_only --template browser_capture_review --template direct_truss_3d`
   - 返回 `unknown headless template "browser_capture_review"`
   - 同时存在模板重复参数输入，提示可能把首参作为 template 解析，命名/参数处理可提升可用性。

4. `--runtime-style hybrid --template browser_submit_then_poll`
   - 返回同类 `unknown headless template`，而该模板在 `hybrid` 列表中存在。

5. `--runtime-style unknown_style --template direct_truss_3d`
   - **未报错**，且成功初始化 workflow（`initialized headless workflow`）。
   - 生成文件里 `template.id='direct_truss_3d'`，`template.runtime_style='service_only'`，说明未知 runtime_style 可能被静默忽略。

### 结论（runtime 部分）
- 明显问题是 `runtime-style` 语义上存在“可用性陷阱”：
  - 与存在模板组合返回的是“unknown template”而非兼容性错误。
  - 非法 `runtime-style` 未验证失败。
- 这会放大用户误操作成本，建议修为更精确的参数验证与错误文案。

## 5) 多场景矩阵（10个模板）结果
模板列表：
`direct_spring_3d`, `direct_acoustic_bar_1d`, `direct_electrostatic_triangle`, `direct_heat_triangle`, `direct_plane_triangle`, `direct_thermal_frame_3d`, `direct_mesh_pipeline`, `material_composite_thermo_electric_panel_screening`, `material_thermo_shield_screening`, `material_structural_panel_screening`。

全部执行：
- `init -> validate -> plan -> run --json`（dry run）
- `run --execute --executor mock --json`（`material_*` 额外加 `--allow-sensitive`）

### 结果
- 所有 10 个模板 dry-run 均 `status=ok`。
- 所有 10 个模板 execute:mock 均 `status=ok`。
- 未出现 new blocker。
- 各 dry-run 的 `executed_step_count` 分布：
  - 直接求解模板：`3`
  - 材料复合/筛选模板：`9`

## 6) 说明
- 与之前轮次一致，`material_study_*` 的敏感确认行为已在先前记录（默认 block，可由 `--allow-sensitive` 放行）。本轮主要目的是 runtime 风格一致性与混合模板覆盖。

## 7) 最佳修复建议（优先级）
1. P1：非法 runtime_style（如 `unknown_style`）应立即报错。
2. P1：template 不匹配时若是 runtime 不兼容，应报错文本明确当前 runtime 与可用模板集合。
3. P2：当 runtime/filter 与 template 冲突时增加可操作的修复提示（如“切换到 ... runtime_style”）。
