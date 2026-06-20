# Kyuubiki Standard Benchmark Comparison

- Profile: `10k`
- Matrices: `mechanical-core`, `thermal-core`, `compound-core`
- Reports directory: `workers/rust/benchmarks/reports`

## Included reports

- `mechanical-core`: `workers/rust/benchmarks/reports/mechanical-core-10k-compare.md`
- `thermal-core`: `workers/rust/benchmarks/reports/thermal-core-10k-compare.md`
- `compound-core`: `workers/rust/benchmarks/reports/compound-core-10k-compare.md`

## mechanical-core

- Profile: `10k`
- Matrix: `mechanical-core`
- Repeat count: `1`
- Generated at (unix): `1781958380`
- Baseline generated at (unix): `1781950228`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `axial-bar-10k` | ok | 0.2895 | +990.88% | 17 MiB | +7.46% |
| `truss-roof-10k` | ok | 857.4654 | +160.23% | 50 MiB | +1.87% |
| `space-frame-10k` | ok | 43.9748 | +232.71% | 56 MiB | -1.44% |
| `plane-panel-10k` | ok | 918.5435 | +178.49% | 69 MiB | -23.50% |
| `plane-quad-panel-10k` | ok | 8430.1088 | +180.73% | 81 MiB | -32.88% |

## thermal-core

- Profile: `10k`
- Matrix: `thermal-core`
- Repeat count: `1`
- Generated at (unix): `1781958388`
- Baseline generated at (unix): `1781950237`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `heat-plane-quad-10k` | ok | 5884.9923 | +125.81% | 20 MiB | +13.36% |

## compound-core

- Profile: `10k`
- Matrix: `compound-core`
- Repeat count: `1`
- Generated at (unix): `1781958401`
- Baseline generated at (unix): `1781950257`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `truss-roof-10k` | ok | 664.2414 | +102.79% | 50 MiB | -2.76% |
| `space-frame-10k` | ok | 30.3316 | +89.33% | 57 MiB | -6.25% |
| `heat-plane-quad-10k` | ok | 5634.3108 | +107.03% | 61 MiB | -4.97% |
| `compound-surface-panel-10k` | ok | 6932.7983 | +105.61% | 72 MiB | -21.68% |
