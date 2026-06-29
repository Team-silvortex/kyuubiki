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
- Generated at (unix): `1782739900`
- Baseline generated at (unix): `1781950228`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `axial-bar-10k` | ok | 0.0853 | +221.34% | 16 MiB | +0.20% |
| `truss-roof-10k` | ok | 281.2433 | -14.65% | 46 MiB | -6.62% |
| `space-frame-10k` | ok | 16.4900 | +24.76% | 57 MiB | +0.14% |
| `plane-panel-10k` | ok | 294.6987 | -10.65% | 68 MiB | -24.99% |
| `plane-quad-panel-10k` | ok | 290.9230 | -90.31% | 78 MiB | -35.58% |
|  | stage rss | `precompute=2.968 ms/74 MiB, assemble_global=4.868 ms/78 MiB, solve_system=279.737 ms/78 MiB, assemble=1.070 ms/78 MiB` |  |  |  |

## thermal-core

- Profile: `10k`
- Matrix: `thermal-core`
- Repeat count: `1`
- Generated at (unix): `1782739900`
- Baseline generated at (unix): `1781950237`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `heat-plane-quad-10k` | ok | 47.7954 | -98.17% | 14 MiB | -19.77% |
|  | stage rss | `precompute=10 MiB, assemble_global=11 MiB, reduce_system=12 MiB, solve_system=13 MiB, assemble=14 MiB` |  |  |  |

## compound-core

- Profile: `10k`
- Matrix: `compound-core`
- Repeat count: `1`
- Generated at (unix): `1782739901`
- Baseline generated at (unix): `1781950257`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `truss-roof-10k` | ok | 290.9939 | -11.16% | 45 MiB | -12.61% |
| `space-frame-10k` | ok | 16.5835 | +3.52% | 56 MiB | -7.91% |
| `heat-plane-quad-10k` | ok | 39.2276 | -98.56% | 56 MiB | -12.97% |
|  | stage rss | `precompute=56 MiB, assemble_global=56 MiB, reduce_system=56 MiB, solve_system=56 MiB, assemble=56 MiB` |  |  |  |
| `compound-surface-panel-10k` | ok | 294.6586 | -91.26% | 68 MiB | -25.69% |
|  | stage rss | `precompute=2.406 ms/63 MiB, assemble_global=5.232 ms/67 MiB, solve_system=284.234 ms/68 MiB, assemble=0.540 ms/68 MiB` |  |  |  |
