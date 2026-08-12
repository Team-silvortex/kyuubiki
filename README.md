# kyuubiki-playground 模块化实验室

本目录用于真实调用 kyuubiki 进行材料研发 / 仿真实验。  
我把旧脚本按“模块化 + 流程标准化”重构成统一入口 `labctl.sh`，保持旧脚本兼容。

## 目录

- `scripts/labctl.sh`：实验室主入口
- `scripts/lib/labctl_common.sh`：通用运行函数（工作区隔离、日志、manifest）
- `scripts/modules/`：按场景拆分的模块（每个模块调用一条历史脚本）
- `runs/`：标准化执行结果目录（运行后自动创建）

## 快速开始

```bash
cd /Users/Shared/chroot/research/kyuubiki-playground
./scripts/labctl.sh list
./scripts/labctl.sh run material-explore
./scripts/labctl.sh run headless-workflow --run-id smoke-001 --label "headless baseline"
./scripts/labctl.sh run headless-template-matrix --set HEADLESS_ROUNDS=2 --set HEADLESS_MAX_VOLTAGE=3200
```

## 运行规范（固定）

1. 每个实验模块一次运行落盘到 `runs/<module>/<run_id>/`。
2. 每次运行都会有：
   - `steps/<step>.out`、`steps/<step>.err`、`steps/<step>.status`
   - `run-manifest.json`
   - `workspace/`（模块隔离工作区，旧脚本产物默认落在这里）
3. 模块运行前后不再手工清理历史目录，结果可追溯。

## 已有模块（与 `scripts/labctl.sh run` 对应）

- `material-explore`
- `headless-workflow`
- `headless-template-matrix`
- `chain-next-regression`
- `headless-fault-injection`
- `boundary-regression`
- `large-mesh-auto-fallback`
- `headless-service-rerun`
- `service-port-matrix`

## 参数化建议

- 常用环境变量：
  - `HEADLESS_ROUNDS`
  - `HEADLESS_START_VOLTAGE`
  - `HEADLESS_MAX_VOLTAGE`
  - `HEADLESS_MIN_VOLTAGE`
  - `SYNC_SDK_FROM_DEV`
- 通过 `--set KEY=VALUE` 传入（示例：`--set HEADLESS_ROUNDS=3`）。

## 兼容说明

本次模块化不改造业务脚本逻辑；它们仍然是实验基线。  
新的价值在于：

- 统一了实验入口
- 统一了每次实验的落盘结构
- 便于你持续补充新模块，扩展更多研究场景（电磁 / 光声 / 力热 / 热声等）
