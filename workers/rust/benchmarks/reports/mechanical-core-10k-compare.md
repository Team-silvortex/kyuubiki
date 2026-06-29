# Kyuubiki Benchmark Comparison

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
