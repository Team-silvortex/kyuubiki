# 1.9 Benchmark Baseline Overview

This page tracks the first standardized performance-baseline ladder for the
`1.9.*` cycle.

## Standard matrices

- `mechanical-core`
  Axial, truss, frame, and plane-mechanical baseline family.
- `thermal-core`
  Thermal conduction baseline family.
- `compound-core`
  Structure-plus-thermal coupled workflow family.

## Scale ladder

- `10k`
  First-line checked baseline tier for local regression work.
- `15k`
  Mid-scale confirmation tier for the same standard matrices.
- `20k`
  Heavy local validation tier before selected `100k` pushes.
- `100k`
  Checked heavy-tier ladder for representative standard matrices.
- `200k`
  Remote-first exploratory tier. Functional coverage is enforced through
  catalog shape tests; timing baselines should be produced on `kyuubiki-lab`
  before they are treated as release evidence.

## Naming rules

- Core historical files remain `10k-baseline.json`, `15k-baseline.json`, and so on.
- Standard matrix files use `<matrix>-<profile>-baseline.json`.
- Reports follow the same pattern under `benchmarks/reports/`.

## Refresh commands

```bash
cd <repo>
make benchmark-standard-baselines PROFILE=10k REPEAT=3
make benchmark-standard-baselines PROFILE=15k REPEAT=3
make benchmark-standard-baselines PROFILE=20k REPEAT=3
make benchmark-profile-remote PROFILE=200k MATRIX=thermal-core REPEAT=1
```

## Current checked baseline set

### `mechanical-core`

| Profile | File | Cases | Heaviest median | Peak RSS |
| --- | --- | ---: | ---: | ---: |
| `10k` | `mechanical-core-10k-baseline.json` | 5 | `plane-quad-panel-10k` `3002.8823 ms` | `122448 KiB` |
| `15k` | `mechanical-core-15k-baseline.json` | 5 | `plane-quad-panel-15k` `10955.1584 ms` | `179536 KiB` |
| `20k` | `mechanical-core-20k-baseline.json` | 5 | `plane-quad-panel-20k` `17771.2455 ms` | `204416 KiB` |
| `100k` | `mechanical-core-100k-baseline.json` | 5 | `plane-quad-panel-100k` `634713.8197 ms` | `974656 KiB` |

### `thermal-core`

| Profile | File | Cases | Heaviest median | Peak RSS |
| --- | --- | ---: | ---: | ---: |
| `10k` | `thermal-core-10k-baseline.json` | 1 | `heat-plane-quad-10k` `2606.2206 ms` | `17728 KiB` |
| `15k` | `thermal-core-15k-baseline.json` | 1 | `heat-plane-quad-15k` `8805.7615 ms` | `26336 KiB` |
| `20k` | `thermal-core-20k-baseline.json` | 1 | `heat-plane-quad-20k` `15196.4042 ms` | `45008 KiB` |
| `100k` | `thermal-core-100k-baseline.json` | 1 | `heat-plane-quad-100k` `272760.5366 ms` | `146372 KiB` |

### `compound-core`

| Profile | File | Cases | Heaviest median | Peak RSS |
| --- | --- | ---: | ---: | ---: |
| `10k` | `compound-core-10k-baseline.json` | 4 | `compound-surface-panel-10k` `3371.7684 ms` | `92848 KiB` |
| `15k` | `compound-core-15k-baseline.json` | 4 | `compound-surface-panel-15k` `8101.7782 ms` | `118064 KiB` |
| `20k` | `compound-core-20k-baseline.json` | 4 | `compound-surface-panel-20k` `20707.0795 ms` | `140672 KiB` |
| `100k` | `compound-core-100k-baseline.json` | 4 | `compound-surface-panel-100k` `288360.2755 ms` | `523184 KiB` |

## Notes

- `10k` is the practical default for local regression gating.
- `15k` and `20k` already form a meaningful scale ladder for `1.9.*`.
- `mechanical-core-100k`, `thermal-core-100k`, and `compound-core-100k` now
  form a complete checked heavy-tier ladder, with the thermal and compound
  runs validated on `kyuubiki-lab`.
- Selected `100k` pushes should continue to target the most representative
  standard matrices instead of trying to expand every path at once.
- `200k` coverage currently means the standard matrix catalog must still
  generate 200k-scale cases, and selected remote smoke runs should be captured
  under `tmp/benchmark-profile/` or promoted to checked baselines only after the
  lab result is stable.
