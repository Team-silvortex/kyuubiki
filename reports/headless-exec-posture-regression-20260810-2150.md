# 2026-08-10 21:50 execution-posture / executor 回归轮

## 目标
- 验证 `--execution-posture` 与 `--executor` 的组合行为
- 复现并归类：参数校验错误、执行失败边界、敏感确认放行流程
- 产出可复用最小回归脚本

## 环境与输入
- 仓库：`/Users/Shared/chroot/dev/kyuubiki`
- 临时工作区：`/private/tmp/kyuubiki_posture_round2`
- 基础模板：`solve_wait_result`、`material_study_envelope_ranking`

## 执行矩阵（带 `--execute`）

### A. `solve_wait_result`
1. `--execute --executor mock`
- 结果：`status=ok`, `mode=execute:mock`, `executed_step_count=4`

2. `--execute --executor mock --execution-posture preview`
- 结果：`status=ok`, `mode=execute:mock`, `executed_step_count=4`

3. `--execute --executor mock --execution-posture research`
- 结果：`status` 非 run（结构化 CLI 错误）
- 错误：`research execution requires --executor service; mock cannot provide a no-mock execution guarantee`
- code：`headless_command_failed`
- 结论：参数层面校验正常、阻止 mock 做 research

4. `--execute --executor service --execution-posture preview --api-base-url http://127.0.0.1:4000`
- 结果：`status=failed`，`mode=execute:service`
- 失败：`kyuubiki.headless.transport_failure`，step1 `service_health`
- Message：`Operation not permitted (os error 1)`
- 结论：环境连通性问题（非参数语义问题）

5. `--execute --executor service --execution-posture research --api-base-url http://127.0.0.1:4000`
- 结果：同上 `transport_failure`

6. `--execute --executor hybrid --execution-posture preview`
- 结果：`status=failed`, `mode=execute:hybrid`
- 失败：`service_health` 阶段 transport_failure（同 127.0.0.1:4000）
- 结论：hybrid 在当前环境也走了服务通道，且未绕过 service 端口连接

### B. `material_study_envelope_ranking`
1. `--execute --executor mock --execution-posture preview`
- 结果：`status=blocked`, `blocked_by_confirmation={'index':1,'risk':'sensitive'}`
- 说明：敏感确认仍然默认阻断

2. `--execute --allow-sensitive --executor mock --execution-posture preview`
- 结果：`status=ok`, `mode=execute:mock`, `executed_step_count=3`
- 说明：`--allow-sensitive` 可放通敏感确认

3. `--execute --allow-sensitive --executor mock --execution-posture research`
- 结果：CLI 校验拒绝（与 mock 同 `research` 限制一致）
- 错误：同上 `research execution requires --executor service; mock cannot provide a no-mock execution guarantee`

## 之前已确认的旁路（本轮顺手复核）
- `--executor mock --execution-posture research` / `--executor service --execution-posture research` 需搭配 `--execute` 才有效；否则返回：`--executor and --execution-posture require --execute`
- `--executor local`：返回 `unsupported executor "local"`

## 可复用最小回归脚本（可放 CI）
```bash
#!/usr/bin/env bash
set -euo pipefail

root="/private/tmp/kyuubiki_posture_round2"
mkdir -p "$root"

solve_wf="$root/solve_wait_result.workflow.json"
study_wf="$root/material_study_ranking.workflow.json"

./scripts/kyuubiki headless init --template solve_wait_result --out "$solve_wf" --json
./scripts/kyuubiki headless init --template material_study_envelope_ranking --out "$study_wf" --json

# 1) mock baseline success
./scripts/kyuubiki headless run "$solve_wf" --execute --executor mock --json --report-out "$root/ok.json"

# 2) posture required-executor validation
if ./scripts/kyuubiki headless run "$solve_wf" --execute --executor mock --execution-posture research --json >/dev/null 2>"$root/neg.txt"; then
  echo "FAIL: expected posture validation error"
  exit 1
fi
cat "$root/neg.txt"

# 3) sensitive confirmation
if ./scripts/kyuubiki headless run "$study_wf" --execute --executor mock --execution-posture preview --json >/dev/null 2>"$root/blocked.txt"; then
  echo "FAIL: expected study block"
  exit 1
fi
./scripts/kyuubiki headless run "$study_wf" --execute --allow-sensitive --executor mock --execution-posture preview --json --report-out "$root/study_ok.json"

echo "POSTURE-REGRESSION PASS"
```

## 结论
- 与“可用性边界”相关：该轮无新增参数逻辑缺陷（核心报错明确）。
- 与“环境连通性”相关：`service/hybrid` 执行会稳定失败于 `127.0.0.1:4000` 端口访问（非功能回归）。
- 建议把这套命令矩阵直接并入 CI 的 smoke 路径，至少覆盖 `command_validation` 与 `transport_failure` 两类。 
