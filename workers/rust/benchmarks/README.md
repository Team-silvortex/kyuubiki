# Benchmark Artifacts

This directory stores checked-in benchmark baselines and generated comparison reports.

- `*-baseline.json`: stable baseline snapshots used by local compare runs and CI regression gates
- `catalog.default.json`: checked-in benchmark catalog/profile spec loaded by default at runtime
- `BASELINE-OVERVIEW.md`: current `1.9.*` standard matrix and scale-ladder summary
- `reports/*.md`: human-readable current-vs-baseline comparison reports

Baseline naming convention:

- `core` matrix keeps the historical file names such as `10k-baseline.json`
- non-core matrices use `<matrix>-<profile>-baseline.json`
- reports follow the same pattern under `reports/`

1.9 standard matrices:

- `mechanical-core`: axial/truss/frame/plane mechanical baseline family
- `thermal-core`: thermal conduction baseline family
- `compound-core`: coupled structure-plus-thermal workflow family

1.9 baseline workflow:

- use `make benchmark-standard-baselines PROFILE=10k REPEAT=3` to refresh the first-line checked baseline set
- expand the same trio to `15k`, `20k`, selected `100k`, and remote `200k` runs as hardware budget allows
- use `make benchmark-standard-compare PROFILE=10k REPEAT=1` to run the standard regression gate trio
- use `make benchmark-standard-report PROFILE=10k REPEAT=1` to emit the per-matrix reports plus one merged summary report
- use `make benchmark-standard-nightly PROFILE=10k REPEAT=1` to run the same trio on `kyuubiki-lab` and pull the reports back locally

Typical commands:

```bash
cd <repo>
make benchmark-baseline PROFILE=10k REPEAT=3
make benchmark-compare PROFILE=10k REPEAT=1
make benchmark-report PROFILE=10k REPEAT=1
make benchmark-baseline PROFILE=100k REPEAT=3
cargo run --release -q -p kyuubiki-benchmark -- --profile 10k --matrix thermal --repeat 3
cargo run --release -q -p kyuubiki-benchmark -- --profile 10k --matrix compound --repeat 1
make benchmark-baseline PROFILE=10k MATRIX=thermal REPEAT=3
make benchmark-baseline PROFILE=10k MATRIX=mechanical-core REPEAT=3
make benchmark-baseline PROFILE=10k MATRIX=compound-core REPEAT=1
make benchmark-standard-baselines PROFILE=10k REPEAT=3
make benchmark-standard-compare PROFILE=10k REPEAT=1
make benchmark-standard-report PROFILE=10k REPEAT=1
make benchmark-profile-remote PROFILE=200k MATRIX=thermal-core REPEAT=1
```

These Make targets run the benchmark crate in `--release` mode so checked-in
baselines and current comparisons stay on the same performance footing.

For the `100k` and `200k` profiles, prefer running on a dedicated remote/Linux
host instead of a laptop-class development machine. A full `repeat=3` baseline
can take significantly longer than the default `10k` tier and may peak above
`500 MiB` RSS depending on the case mix. Use `make benchmark-profile-remote`
for 200k smoke coverage before promoting a matrix into the checked regression
baseline set.

Current regression-gate default:

- cases with baseline median below `5.0 ms` are still reported, but are not
  treated as hard regression failures by default
- `make benchmark-standard-compare` uses the same default gate thresholds as
  `make benchmark-compare`: `+25%` median, `+20%` peak RSS, and `5.0 ms`
  minimum baseline median

That keeps tiny micro-benchmarks such as the axial bar cases visible without
letting timer noise dominate the gate.
