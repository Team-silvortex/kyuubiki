# Kyuubiki 研究课题扩展轮次报告（2026-08-10）

## 目标
扩展研发课题到“全模板 + 混合场景（浏览器/编排）”，用无头 Headless SDK 验证平台端到端流程可运行性。

## 1) 全量模板基线（37 个，去重后）
- 执行命令：`bash scripts/run-headless-template-matrix.sh --mode both --executor mock`
- 工作目录：`/tmp/kyuubiki-template-matrix-expanded-20260810-round2b`
- 报告：`/Users/Shared/chroot/dev/kyuubiki/reports/headless-template-matrix-expanded-round2b-20260810-20260810-155953.md`

### 结果统计
- `total`: 37
- `passed`: 32
- `blocked`: 5
- `failed`: 0
- `dry_ok`: 32
- `exec_ok`: 32

### 阻断模板（5 个）
- `browser_capture_review`
- `browser_submit_then_poll`
- `workflow_submit_monitor`
- `material_study_envelope_ranking`
- `material_study_envelope_catalog`

这些模板在无 `--allow-sensitive` 下都进入 `blocked`，但 `dry/execute` 验证均为 `ok` 且 `validate_issue_count=0`，更像是策略闸门，不是模板语义错误。

## 2) “敏感动作放行”复测（5 个阻断模板）
对上面 5 个模板补跑 `--allow-sensitive`。

### 结果
- 全部 5 个模板在 dry/execute 下均 `status=ok`
- `browser_*` 两个模板：`required_engines` 含 `browser`，且 `needs_desktop_browser=true`
- `workflow_submit_monitor`、`material_*` 两类：`required_engines` 含 `service`，无桌面浏览器依赖
- 仍保持 `risk_counts` 中 `sensitive`>0（符合 policy 预期）

## 3) 结论（研发/稳定性）
1. 平台执行链路在 mock executor 下，机械/热/电/材料大类模板表现稳定。
2. 当前阻断是策略层面（sensitive + browser 依赖）导致；不是解析、plan/render 的直接缺陷。
3. 若目标是“研发可落地”而非严格沙盒保守，可考虑默认行为：
   - 在受控环境下允许 `--allow-sensitive`
   - 对 `needs_desktop_browser=true` 的模板，给出前置环境校验（本地浏览器、display/Xvfb）

## 参考文件
- [round2b 报告](/Users/Shared/chroot/dev/kyuubiki/reports/headless-template-matrix-expanded-round2b-20260810-20260810-155953.md)
- [round2a（重复项导致 inflated）报告](/Users/Shared/chroot/dev/kyuubiki/reports/headless-template-matrix-expanded-round2-20260810-20260810-155402.md)
- [本轮总结数据](/tmp/kyuubiki-template-matrix-expanded-20260810-round2b/summary.json)
