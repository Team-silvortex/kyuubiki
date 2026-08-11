# Headless research round11: multimodal direct-template stress test (2026-08-10)

## Goal
- 验证 kyuubiki 在无头 service 模式下的跨物理域执行（电磁/热/热力/声学/材料复合）是否可用、可重复，并观察参数扰动是否真实传导。

## Inputs
- 基线与变更输入来自：
  - `tmp/research-round11-multiphysics/inputs/{material_dielectric_base,direct_electrostatic_base,direct_heat_base,direct_thermal_frame3d_base,direct_acoustic_base}.json`
- 实验用例输出目录：
  - `tmp/research-round11-multiphysics/results/{electro-baseline,electro-highv,heat-baseline,heat-thick,thermal-baseline,thermal-hot,acoustic-baseline,acoustic-hi,dielectric-baseline,dielectric-hv}`
- 使用 `--executor service --execution-posture research --execute --allow-sensitive`。

## Cases
- `electro-baseline`: `direct_electrostatic_base`（基线）
- `electro-highv`: `direct_electrostatic` 将边界电位提高为 100V，厚度 0.05→0.1
- `heat-baseline`: `direct_heat_base`
- `heat-thick`: `direct_heat` 将热边界 `h1.temperature` 由 0→60，厚度 0.02→0.1
- `thermal-baseline`: `direct_thermal_frame_3d`
- `thermal-hot`: `direct_thermal_frame_3d` 增加约束端载荷 `n1.load_x=450, load_y=1500`
- `acoustic-baseline`: `direct_acoustic_bar_1d`
- `acoustic-hi`: 声学频率 440Hz→2500Hz，体积速度源 0.003→0.012
- `dielectric-baseline`: `material_dielectric_screening`（基线）
- `dielectric-hv`: `material_dielectric_screening` 中边界电压 1200V→2500V，介电层介电常数 3.0104e-11→4.7，厚度 0.001→0.0015

> 注意：首次执行时 `thermal-hot` 首轮出现 transport 连接失败，重试后成功；说明偶发网络/服务侧抖动需要记录。

## Execution status summary
- 10 次 run 中 9 次初始成功，1 次重试成功后最终全部成功（最后结果都为 `status: ok`）。
- 单次作业运行时延大都在 ~1.0s 左右（`execution_elapsed_ms` 在 1011–1022ms）。

## Metric snapshot（每类 case）

### Electrostatic (direct)
- `electro-baseline`
  - `max_electric_field = 7.9057`
  - `max_flux_density = 19.7642`
- `electro-highv`
  - `max_electric_field = 79.0569`
  - `max_flux_density = 197.6424`
  - 约 10x 提升，参数扰动可见（来自电位与厚度改动）。

### Heat
- `heat-baseline`
  - `max_temperature = 100.0`
  - `max_heat_flux = 2846.05`
- `heat-thick`
  - `max_temperature = 100.0`
  - `max_heat_flux = 2490.29`
  - 厚度和边界温度调整后热通量降低。

### Thermal frame 3D（关键）
- `thermal-baseline`
  - `max_stress = 123,900,000`
  - `max_displacement = 0.0`
  - `max_temperature_delta = 35.0`
  - `max_moment = 2688`
- `thermal-hot`（含重试）
  - `max_stress = 123,900,000`
  - `max_displacement = 0.0`
  - `max_temperature_delta = 35.0`
  - `max_moment = 2688`
  - 与基线几乎完全一致：这次载荷施加在约束节点，系统表现为静态约束下无位移/应变放大。

### Acoustic
- `acoustic-baseline`
  - `frequency = 440 Hz`
  - `max_pressure = 4.9872`
  - `max_spl_db = 107.94 dB`
  - `max_acoustic_intensity = 0.001762`
- `acoustic-hi`
  - `frequency = 2500 Hz`
  - `max_pressure = 3.4848`
  - `max_spl_db = 104.82 dB`
  - `max_acoustic_intensity = 0.0001448`
  - 高频+高源幅值下压力和强度反降，验证模型参数链路可见。

### Composite dielectric screening
- `dielectric-baseline`（排序）
  1. `polyimide_film` score=0.800, safety=10000, emax=30000
  2. `ptfe` score=0.549, safety=2000, emax=30000
  3. `alumina_96` score=0.518, safety=4333.33, emax=30000
- `dielectric-hv`（排序）
  1. `alumina_96` score=0.602, safety=4333.33, emax=30000
  2. `polyimide_film` score=0.550, safety=6666.67, emax=45000
  3. `ptfe` score=0.549, safety=2000, emax=30000
  - 这组参数扰动触发了排名重排。

## Finding（研发/稳定性）
- 平台层面可执行性：service 模式下跨域模板可稳定跑通。
- 参数敏感性：多数 case 的核心指标随输入变动变化明显；材料筛选可出现排序反转。
- 可靠性观察：
  - `thermal-hot` 首次报 `kyuubiki.headless.transport_failure`（step1）后，立即重试可恢复。
  - 对模型而言，加载约束节点会导致热力结果不变（可视为边界设置问题，建议在模型设计阶段把“加载节点可动性”与“约束节点”显式分离）。

## Artifacts
- 输入：`tmp/research-round11-multiphysics/inputs/`
- 结果：`tmp/research-round11-multiphysics/results/`
- 日志含 `run_stdout.json`, `run_stderr.txt`, `run_raw.json`, `material_report.json`（材料工况）
