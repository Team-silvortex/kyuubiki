# Headless Complexity Round - 2026-08-10 15:50

- 目标: 继续扩大 `--executor mock` 的复杂 workflow 研究覆盖（高节点、跨域、多场景）
- 环境: `/Users/Shared/chroot/dev/kyuubiki`
- 说明: 使用本地 `headless validate` + `headless run --executor mock`

## 场景集合

1) `complex_heat_chain_60`
- 生成方式: 基于 `direct_heat_triangle` 的 `solve_heat_plane_triangle_2d` 步骤复制为 20 组
- 总步数: 60
- 验证文件: `/tmp/kyuubiki-complex-round/complex_heat_chain_60.validate.json`
- 执行文件: `/tmp/kyuubiki-complex-round/complex_heat_chain_60.mock.out`
- 结果: `validate ok=true`，`run status=ok`，`mode=execute:mock`，`executed_step_count=60`

2) `complex_thermal_chain_45`
- 生成方式: 基于 `direct_thermal_triangle` 的 `solve_thermal_*` 步骤复制为 15 组
- 总步数: 45
- 验证文件: `/tmp/kyuubiki-complex-round/complex_thermal_chain_45.validate.json`
- 执行文件: `/tmp/kyuubiki-complex-round/complex_thermal_chain_45.mock.out`
- 结果: `validate ok=true`，`run status=ok`，`mode=execute:mock`，`executed_step_count=45`

3) `complex_mesh_chain_54`
- 生成方式: 基于 `direct_mesh_pipeline` 的 `direct_mesh_solve` 步骤复制为 18 组
- 总步数: 54
- 验证文件: `/tmp/kyuubiki-complex-round/complex_mesh_chain_54.validate.json`
- 执行文件: `/tmp/kyuubiki-complex-round/complex_mesh_chain_54.mock.out`
- 结果: `validate ok=true`，`run status=ok`，`mode=execute:mock`，`executed_step_count=54`

4) `complex_mixed_chain_72`
- 生成方式: 热-热力-声学-网格 4 模式交替，每种各 6 轮（共 24 个 solve 组）
- 总步数: 72
- 验证文件: `/tmp/kyuubiki-complex-round/complex_mixed_chain_72.validate.json`
- 执行文件: `/tmp/kyuubiki-complex-round/complex_mixed_chain_72.mock.out`
- 结果: `validate ok=true`，`run status=ok`，`mode=execute:mock`，`executed_step_count=72`

## 结果摘要（已落库）
- 汇总文件: `/tmp/kyuubiki-complex-round/complex_scenarios_summary.json`
- 全部 4 个复杂场景均通过 schema 验证与 mock 执行

## 关键结论
- `mock` 路径下，`headless` 对大规模 chain 的执行稳定，能覆盖 60+ 节点规模
- 复杂度来自于步数与异构 action 混合，并未触发额外 schema 报错
- 与此前服务端口受限问题无关（服务端仍需可用 `orchestrator/frontend` 才能继续真实端口链路回归）

## 与以往回归的区别
- 这一次重点是“高步数 + 高异构 + 快速可复用”组合；相较之前的单模板回归，新增了更大规模和跨 action 的组合压力样例，可直接用于后续回归基线。
