# Kyuubiki Benchmark Comparison

- Profile: `10k`
- Matrix: `thermal-core`
- Repeat count: `1`
- Generated at (unix): `1782739900`
- Baseline generated at (unix): `1781950237`

| Case | Status | Median ms | Delta | Peak RSS | Delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `heat-plane-quad-10k` | ok | 47.7954 | -98.17% | 14 MiB | -19.77% |
|  | stage rss | `precompute=10 MiB, assemble_global=11 MiB, reduce_system=12 MiB, solve_system=13 MiB, assemble=14 MiB` |  |  |  |
