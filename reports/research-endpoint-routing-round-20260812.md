# 端点路由与大网格回归（2026-08-12）

- 时间: 2026-08-12 14:22:00
- 目标: 继续验证大规模输入的 service fallback 路由行为（3999/3000/4000）与运行可复现性
- 运行目录: `/Users/Shared/chroot/dev/kyuubiki`

## 变更
- 在 `scripts/run_headless_research_matrix.sh` 增加控制面参数与自动路由逻辑：
  - 新增 `--service-control-plane-url`
  - 当主执行失败且判定为 artifact/transport 类失败时，fallback endpoint 自动切到控制面
  - 报表记录 `service_fallback_api` 与 `service_fallback_control_plane_api`
- 新增超大输入夹具：`results/sdk-large-mesh-1m/input_3000x1000_noids_jobwait_1200000.json`
  - 规模约为 3,000,000 元素，约 3,006,003 节点
  - 文件大小约 704.5MB
- 在 `scripts/run_service_port_matrix_copy.sh` 新增用例：`large_3000x1000_noids`
- 新增规模补点输入：
  - `results/sdk-large-mesh-1m/input_2000x1000_noids_jobwait_1200000.json`
    - 规模 2,000,000 元素，约 2,003,001 节点
  - `results/sdk-large-mesh-1m/input_2000x2000_noids_jobwait_1200000.json`
    - 规模 4,000,000 元素，约 4,004,001 节点
  - `results/sdk-large-mesh-1m/input_2500x1600_noids_jobwait_1200000.json`
    - 规模 4,000,000 元素，约 4,004,101 节点
  - `results/sdk-large-mesh-1m/input_1800x2200_noids_jobwait_1200000.json`
    - 规模 3,960,000 元素，约 3,964,001 节点
  - `results/sdk-large-mesh-1m/input_3000x900_noids_jobwait_1200000.json`
    - 规模 2,700,000 元素，约 2,703,901 节点

## 关键执行结果

### 1) 端口矩阵（`scripts/run_service_port_matrix_copy.sh`）
- 报告: `reports/service-matrix-port-rotation-20260812-141604.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-141604/`

- 复测: `reports/service-matrix-port-rotation-20260812-151600.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-151600/`
  - 修复已生效：`fallback-triggered` 不再只看 `service_3000` 进程退出码，也会因 transport/artifact 标识触发 fallback。
  - `service_4000.fallback-url` 统一在发生退化时落到 `http://127.0.0.1:4000`
  - `large_3000x1000_noids` 中 `service_4000.fallback-reason=transport_or_artifact_signature`，且保留 size/limit 上报

- 扩展复测 1：`reports/service-matrix-port-rotation-20260812-153000.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-153000/`
- 参数：`SERVICE_PRIMARY_API_BASE_URL=3999`, `SERVICE_FALLBACK_API_BASE_URL=3000`, `SERVICE_CONTROL_PLANE_API_BASE_URL=4000`
- 关键现象：
  - `large_700x700` / `large_1000x1000_noids`：`service_3000` 都返回 `kyuubiki.headless.transport_failure`（被判定为 transport/artifact signature，触发 fallback）
  - fallback 到 4000 后 `small/medium` 全部恢复成功（report status=ok）
  - `large_3000x1000_noids`：`service_4000` 仍失败，`size_bytes=728884250`, `limit_bytes=536870912`

- 扩展复测 2：`reports/service-matrix-port-rotation-20260812-153300.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-153300/`
- 参数：`SERVICE_PRIMARY_API_BASE_URL=3000`, `SERVICE_FALLBACK_API_BASE_URL=3999`, `SERVICE_CONTROL_PLANE_API_BASE_URL=3999`
- 关键现象：
  - `large_700x700` / `large_1000x1000_noids`：`service_3000` 通过（无 fallback）
  - `large_3000x1000_noids`：继续触发 artifact 类退化并 fallback 到 3999，但 3999 仍限额失败

- 扩展复测 3：`reports/service-matrix-port-rotation-20260812-155700.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-155700/`
- 参数：`SERVICE_MATRIX_TS=20260812-155700`（默认端口：3000->4000）
- 关键现象（在修正 2M/4M 输入几何后）
  - `large_2000x1000_noids`：`service_3000` 触发退化，fallback 到 `4000` 后恢复为 `ok`
  - `large_2000x2000_noids`：`service_3000` 触发退化，`service_4000` 仍 `runtime_failure`
    - `size_bytes=997085560 limit_bytes=536870912`
  - `large_3000x1000_noids` 行为保持与此前一致：`service_4000` 仍失败

- 扩展复测 4：`reports/service-matrix-port-rotation-20260812-162200.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-162200/`
- 参数：`SERVICE_MATRIX_TS=20260812-162200`（默认端口：3000->4000）
- 关键现象（2500x1600 点补齐边界）：
  - `large_2500x1600_noids`：`service_3000` 触发 `frontend_proxy_artifact_limit`，`4000` 复测仍失败
    - `size_bytes=979,309,961`（`limit_bytes=536,870,912`）
  - `large_2000x2000_noids` 行为一致，`4000` 仍失败
  - `large_3000x1000_noids` 行为继续一致

- 扩展复测 5：`reports/service-matrix-port-rotation-20260812-164000.md`
- 结果目录: `results/service-matrix-port-rotation-20260812-164000/`
- 参数：`SERVICE_MATRIX_TS=20260812-164000`（默认端口：3000->4000）
- 关键现象（形状敏感性补点）：
  - `large_1800x2200_noids`：`service_3000` 触发 `frontend_proxy_artifact_limit`，`4000` 仍失败
    - `size_bytes=1,044,236,745`（`limit_bytes=536,870,912`）
  - `large_3000x900_noids`：`service_3000` 触发 `frontend_proxy_artifact_limit`，`4000` 仍失败
    - `size_bytes=702,009,782`（`limit_bytes=536,870,912`）
- `large_700x700`
  - 3000 成功
  - 无 fallback
- `large_1000x1000_noids`
  - 3000 成功
  - 无 fallback
- `large_3000x1000_noids`
  - `service_3000` 失败：`frontend_proxy_artifact_limit`，并建议使用 control-plane
  - `service_4000` fallback 触发且仍失败：
    - `headless.execution` 错误：`direct FEM model exceeds artifact transport limit`
    - `size_bytes=728884250 limit_bytes=536870912`
  - `large_2000x1000_noids`
    - `service_3000` 失败：`frontend_proxy_artifact_limit`，fallback 到 4000 成功
  - `large_2000x2000_noids`
    - `service_3000` 失败：`frontend_proxy_artifact_limit`
    - `service_4000` fallback 触发且仍失败：
      - `headless.execution` 错误：`direct FEM model exceeds artifact transport limit`
      - `size_bytes=997085560 limit_bytes=536870912`
  - `large_2500x1600_noids`
    - `service_3000` 失败：`frontend_proxy_artifact_limit`
    - `service_4000` fallback 触发且仍失败：
      - `headless.execution` 错误：`direct FEM model exceeds artifact transport limit`
      - `size_bytes=979309961 limit_bytes=536870912`
  - `large_1800x2200_noids`
    - `service_3000` 失败：`frontend_proxy_artifact_limit`
    - `service_4000` fallback 触发且仍失败：
      - `headless.execution` 错误：`direct FEM model exceeds artifact transport limit`
      - `size_bytes=1044236745 limit_bytes=536870912`
  - `large_3000x900_noids`
    - `service_3000` 失败：`frontend_proxy_artifact_limit`
    - `service_4000` fallback 触发且仍失败：
      - `headless.execution` 错误：`direct FEM model exceeds artifact transport limit`
      - `size_bytes=702009782 limit_bytes=536870912`

### 2) 语义控制面回退验证（`scripts/run_headless_research_matrix.sh`）
- 命令：
  `bash scripts/run_headless_research_matrix.sh --pipeline service --template material_dielectric_screening --service-primary-url http://127.0.0.1:3999 --service-fallback-url http://127.0.0.1:3000 --service-control-plane-url http://127.0.0.1:4000`
- 报告: `reports/headless-research-matrix-20260812-142116.md`
- 观察：
  - `service_primary`（3999）失败：`kyuubiki.headless.transport_failure`
  - `service_fallback`（路由到 4000）成功：`ok`

## 当前可见问题与建议
1. 3M 输入可被识别为 transport/artifact 类型错误，但控制面 4000 当前仍有限额 512MB，导致 700MB 左右负载仍会失败。
2. `run_service_port_matrix_copy.sh` 的 3000 退化路由策略已和 `run_headless_research_matrix.sh` 对齐，剩余问题是 `control-plane` 侧仍被 512MB 限制卡住 3M 规模输入。
3. 现有 700/1000 级样例可稳定通过，但 3M 级别达到实际系统容量边界，建议作为“限界回归”保留。
