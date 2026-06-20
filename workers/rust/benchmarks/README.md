# Benchmark Artifacts

This directory stores checked-in benchmark baselines and generated comparison reports.

- `*-baseline.json`: stable baseline snapshots used by local compare runs and CI regression gates
- `reports/*.md`: human-readable current-vs-baseline comparison reports

Typical commands:

```bash
cd <repo>
make benchmark-baseline PROFILE=10k REPEAT=3
make benchmark-compare PROFILE=10k REPEAT=1
make benchmark-report PROFILE=10k REPEAT=1
```

These Make targets run the benchmark crate in `--release` mode so checked-in
baselines and current comparisons stay on the same performance footing.

Current regression-gate default:

- cases with baseline median below `5.0 ms` are still reported, but are not
  treated as hard regression failures by default

That keeps tiny micro-benchmarks such as the axial bar cases visible without
letting timer noise dominate the gate.
