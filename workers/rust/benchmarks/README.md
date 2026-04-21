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
