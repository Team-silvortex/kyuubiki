# Service Port Rotation Regression

- 时间: 2026-08-13 18:29:12
- 仓库: /Users/Shared/chroot/dev/kyuubiki
- 工作目录: /Users/Shared/chroot/dev/kyuubiki
- 说明: 对同一批输入分别执行 3000 与 4000，检查 3000 的 payload 限制退化是否可被 4000 回放。

## large_700x700
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_700x700.json
- validate.status: 0
- service_3000.shell-status: 0
- service_3000.report.status: ok
- service_3000.mode: execute:service
- service_3000.error_code: n/a
- service_3000.body-limit-signature: 0
- service_4000.fallback-triggered: 0
- service_4000.fallback-reason: -
- service_4000.fallback-url: http://127.0.0.1:4000
- service_4000.shell-status: fallback skipped: case=large_700x700
- service_4000.report.status: missing
- service_4000.error_code: n/a
- service_4000.size_bytes: n/a
- service_4000.limit_bytes: n/a
- service_4000.mode: missing

## large_1000x1000_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_1000x1000_noids.json
- validate.status: 0
- service_3000.shell-status: 0
- service_3000.report.status: ok
- service_3000.mode: execute:service
- service_3000.error_code: n/a
- service_3000.body-limit-signature: 0
- service_4000.fallback-triggered: 0
- service_4000.fallback-reason: -
- service_4000.fallback-url: http://127.0.0.1:4000
- service_4000.shell-status: fallback skipped: case=large_1000x1000_noids
- service_4000.report.status: missing
- service_4000.error_code: n/a
- service_4000.size_bytes: n/a
- service_4000.limit_bytes: n/a
- service_4000.mode: missing

## large_3000x1000_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_3000x1000_noids_jobwait_1200000.json
- error: input not found
## large_2000x1000_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_2000x1000_noids_jobwait_1200000.json
- error: input not found
## large_2000x2000_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_2000x2000_noids_jobwait_1200000.json
- error: input not found
## large_2500x1600_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_2500x1600_noids_jobwait_1200000.json
- error: input not found
## large_1800x2200_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_1800x2200_noids_jobwait_1200000.json
- error: input not found
## large_3000x900_noids
- input: /Users/Shared/chroot/dev/kyuubiki/results/sdk-large-mesh-1m/input_3000x900_noids_jobwait_1200000.json
- error: input not found
## small_direct_heat_triangle
- input: /Users/Shared/chroot/dev/kyuubiki/results/headless-all-dryrun-20260804-123750/direct_heat_triangle/input.json
- validate.status: 0
- service_3000.shell-status: 0
- service_3000.report.status: ok
- service_3000.mode: execute:service
- service_3000.error_code: n/a
- service_3000.body-limit-signature: 0
- service_4000.fallback-triggered: 0
- service_4000.fallback-reason: -
- service_4000.fallback-url: http://127.0.0.1:4000
- service_4000.shell-status: fallback skipped: case=small_direct_heat_triangle
- service_4000.report.status: missing
- service_4000.error_code: n/a
- service_4000.size_bytes: n/a
- service_4000.limit_bytes: n/a
- service_4000.mode: missing


---

## 汇总（Port Matrix）
- 文件: `service_3000` 与 `service_4000` 的 shell status 与 report status 已逐项记录。
- 若 `service_3000` 出现 `frontend_proxy_artifact_limit` 或类似 body/transport 限制特征，需路由到 4000（控制面）。
