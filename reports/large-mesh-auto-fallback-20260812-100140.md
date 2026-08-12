# 自动回退验证（大模型体量 -> 3000失败后切换4000）

- 时间: 2026-08-12 10:01:40
- 工作目录: /Users/Shared/chroot/dev/kyuubiki
- 目标: 验证大输入在 3000 执行失败时自动回退 4000 的闭环能力。

## 运行摘要
- input_700x700_jobwait_1200000: 3000 body-limit-like failure detected, auto fallback to 4000
### input_700x700_jobwait_1200000
- validate_rc: 0
- 3000: status=failed, mode=execute:service, steps=0, rc=1, duration=16s
- 4000: status=failed, mode=execute:service, steps=0, rc=1, duration=25s
- 3000 summary: port=3000 rc=1 duration_seconds=16 status=failed error_code=kyuubiki.headless.frontend_proxy_artifact_limit mode=execute:service executed_step_count=0 err_or_message=frontend_proxy_artifact_limit: direct FEM model requires artifact transport: entity_count=981401; connect headless to the runtime control-plane endpoint (default http://127.0.0.1:4000), not the local GUI frontend 
- 4000 summary: port=4000 rc=1 duration_seconds=25 status=failed error_code=kyuubiki.headless.transport_failure mode=execute:service executed_step_count=0 err_or_message=failed to connect to 127.0.0.1:4000 for model artifact upload after 1 bounded attempt(s) with 10000 ms per-address timeout: Operation not permitted (os error 1) 
- 4000 err_or_message(raw): failed to connect to 127.0.0.1:4000 for model artifact upload after 1 bounded attempt(s) with 10000 ms per-address timeout: Operation not permitted (os error 1)

- input_1000x1000_noids_jobwait_1200000: 3000 body-limit-like failure detected, auto fallback to 4000
### input_1000x1000_noids_jobwait_1200000
- validate_rc: 0
- 3000: status=failed, mode=execute:service, steps=0, rc=1, duration=39s
- 4000: status=failed, mode=execute:service, steps=0, rc=1, duration=51s
- 3000 summary: port=3000 rc=1 duration_seconds=39 status=failed error_code=kyuubiki.headless.frontend_proxy_artifact_limit mode=execute:service executed_step_count=0 err_or_message=frontend_proxy_artifact_limit: direct FEM model requires artifact transport: entity_count=2002001; connect headless to the runtime control-plane endpoint (default http://127.0.0.1:4000), not the local GUI frontend 
- 4000 summary: port=4000 rc=1 duration_seconds=51 status=failed error_code=kyuubiki.headless.transport_failure mode=execute:service executed_step_count=0 err_or_message=failed to connect to 127.0.0.1:4000 for model artifact upload after 1 bounded attempt(s) with 10000 ms per-address timeout: Operation not permitted (os error 1) 
- 4000 err_or_message(raw): failed to connect to 127.0.0.1:4000 for model artifact upload after 1 bounded attempt(s) with 10000 ms per-address timeout: Operation not permitted (os error 1)

