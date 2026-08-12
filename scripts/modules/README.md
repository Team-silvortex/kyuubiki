## kyuubiki-playground 模块说明

模块目录采用“一个研究场景一个模块”的结构，每个模块文件只做三件事：

- 声明模块元信息（名称、用途、产物）
- 定义 `run_module()`，调用对应的旧脚本
- 依赖 `labctl_common.sh` 提供通用执行机制

### 通用字段

- `MODULE_NAME`: 人类可读的模块名（与 `labctl.sh run` 入参一致）
- `MODULE_DESCRIPTION`: 模块说明
- `MODULE_LEGACY_SCRIPT`: 复用的旧脚本文件名
- `run_module <run_root> <run_id> <workspace_dir>`: 实际执行入口

### 已注册模块

- `material-explore` -> `run_dielectric_screening.sh`
- `headless-workflow` -> `run_headless_workflow_regression.sh`
- `headless-template-matrix` -> `run_headless_template_matrix.sh`
- `chain-next-regression` -> `run_chain_next_regression.sh`
- `headless-fault-injection` -> `run_headless_fault_injection_regression.sh`
- `boundary-regression` -> `run_material_explore_boundary_regression.sh`
- `large-mesh-auto-fallback` -> `run_large_mesh_auto_fallback.sh`
- `headless-service-rerun` -> `run_headless_service_rerun_fixed.sh`
- `service-port-matrix` -> `run_service_port_matrix.sh`

### 运行标准

1. `./labctl.sh list`：列出模块能力
2. `./labctl.sh run <module>`：执行模块
3. 输出落在 `runs/<module>/<run_id>/`
4. 每次运行都会生成：
   - `steps/<step>.out|.err|.status`
   - `run-manifest.json`（运行元信息）
   - 模块工作区 `workspace/` 下对应脚本产物

### 设计约束

- 旧脚本仍保留，避免大规模重写
- 模块通过标准入口调用旧脚本，便于快速把新实验“入队列”
- 推荐在 `--set` 中显式指定环境变量（例如 `HEADLESS_ROUNDS`）保证复现性
