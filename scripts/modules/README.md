## 保留的研究回归包装模块

这里不是产品的算子库或架构模块注册表，只保留仍有实际脚本入口的研究回归包装。
原先从研究工作区复制、但目标脚本已不存在的八个包装已移除；不再把它们列成可运行能力。
产品自动化优先使用 `scripts/kyuubiki` 的原生入口及 `make` 回归目标。

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

- `headless-workflow` -> `run_headless_workflow_regression.sh`
- `headless-research-matrix` -> `run_headless_research_matrix.sh`

### 运行标准

1. `bash scripts/labctl.sh list`：列出保留的研究包装
2. `bash scripts/labctl.sh run <module>`：执行模块
3. 输出落在 `runs/<module>/<run_id>/`
4. 每次运行都会生成：
   - `steps/<step>.out|.err|.status`
   - `run-manifest.json`（运行元信息）
   - 模块工作区 `workspace/` 下对应脚本产物

### 设计约束

- 不注册指向缺失脚本的模块，不复制产品原生执行逻辑
- 这两个研究包装不代表产品全部功能或回归覆盖率
- 推荐在 `--set` 中显式指定环境变量（例如 `HEADLESS_ROUNDS`）保证复现性
