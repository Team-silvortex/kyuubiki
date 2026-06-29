# Kyuubiki Benchmark Comparison

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
