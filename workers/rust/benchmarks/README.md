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
- expand the same trio to `15k`, `20k`, selected `100k`, and remote `200k`/`300k`/`400k` runs as hardware budget allows
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
cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix extended-physics --repeat 1
cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix structural-extended --repeat 1
cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix thermal-structural --repeat 1
cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix physics-coverage --repeat 1
make benchmark-baseline PROFILE=10k MATRIX=thermal REPEAT=3
make benchmark-baseline PROFILE=10k MATRIX=mechanical-core REPEAT=3
make benchmark-baseline PROFILE=10k MATRIX=compound-core REPEAT=1
make benchmark-standard-baselines PROFILE=10k REPEAT=3
make benchmark-standard-compare PROFILE=10k REPEAT=1
make benchmark-standard-report PROFILE=10k REPEAT=1
make benchmark-profile-remote PROFILE=300k MATRIX=thermal-core REPEAT=1
make benchmark-profile-remote PROFILE=300k MATRIX=thermal-structural REPEAT=1
make benchmark-profile-remote PROFILE=300k MATRIX=mechanical-core CASE=axial-bar-300k REPEAT=1
make benchmark-profile-remote PROFILE=300k MATRIX=mechanical-core CASE=truss-roof-300k REPEAT=1 SOLVER_PRECONDITIONER=all
make benchmark-profile-remote PROFILE=400k MATRIX=thermal-core CASE=heat-plane-quad-400k REPEAT=1
make benchmark-profile-remote PROFILE=400k MATRIX=mechanical-core CASE=axial-bar-400k REPEAT=1
make benchmark-profile-remote PROFILE=400k MATRIX=mechanical-core CASE=truss-roof-400k REPEAT=1 SOLVER_PRECONDITIONER=all
make benchmark-profile-remote PROFILE=400k MATRIX=thermal-structural CASE=thermal-plane-triangle-400k REPEAT=1 SOLVER_PRECONDITIONER=auto
make benchmark-profile-remote PROFILE=400k MATRIX=thermal-structural CASE=thermal-plane-quad-400k REPEAT=1 SOLVER_PRECONDITIONER=auto
make benchmark-profile-remote PROFILE=400k MATRIX=thermal-structural REPEAT=1 SOLVER_PRECONDITIONER=auto
make benchmark-profile-report PROFILE=400k MATRIX=thermal-structural OUTPUT_SLUG=thermal-structural-400k-auto-full
make benchmark-profile-index
```

These Make targets run the benchmark crate in `--release` mode so checked-in
baselines and current comparisons stay on the same performance footing.

The `extended-physics` matrix is the first broad-coverage smoke lane for
modules that were previously only covered by unit or workflow tests. It covers
1D heat, electrostatic, magnetostatic, acoustic, and torsion cases plus 2D heat
triangle, electrostatic triangle/quad, magnetostatic triangle/quad, and Stokes
quad cases. It also includes a 1D advection-diffusion transport case for
concentration-field smoke coverage.

The `structural-extended` matrix covers structural modules outside the standard
mechanical trio: spring 1D/2D/3D, nonlinear spring, contact gap, beam, thermal
beam, and modal frame 2D/3D cases.

The `thermal-structural` matrix covers coupled thermal deformation and static
frame families that need continuous performance visibility: thermal bar,
thermal truss 2D/3D, thermal plane triangle/quad, static frame 2D/3D, and
thermal frame 2D/3D cases.

The `physics-coverage` matrix is the `1.15.x` broad smoke lane. It intentionally
pulls every built-in benchmark template into one medium-scale run so the
project can prepare the `1.15.x` engine/operator SDK contracts and the `1.16.x`
executable task file format against real examples from the major physics
families.

For the `100k`, `200k`, `300k`, and `400k` profiles, prefer running on a dedicated remote/Linux
host instead of a laptop-class development machine. A full `repeat=3` baseline
can take significantly longer than the default `10k` tier and may peak above
`500 MiB` RSS depending on the case mix. Use `make benchmark-profile-remote`
for 200k/300k/400k smoke coverage before promoting a matrix into the checked
regression baseline set. Remote profile runs pass `--progress` so each case
start/done line is visible over SSH while JSON output still lands in the
configured report file.

Initial `400k` lab smokes show that `heat-plane-quad-400k` is a practical
first pressure probe, while `truss-roof-400k` is a heavier iterative-solver
probe. For truss cases, keep `SOLVER_PRECONDITIONER=all` during exploration:
the symmetric Gauss-Seidel strategy can materially reduce iteration count
without materially changing peak RSS.

Current `400k` exploratory lab evidence:

| Case | Median ms | Peak RSS MiB | Notes |
|---|---:|---:|---|
| `axial-bar-400k` | 10.828 | 404.2 | Cheapest mechanical sanity probe |
| `heat-plane-quad-400k` | 6837.637 | 472.9 | Practical first thermal surface probe |
| `truss-roof-400k#jacobi` | 76146.610 | 1375.0 | Heavy iterative structural probe |
| `truss-roof-400k#symmetric-gauss-seidel` | 60873.403 | 1376.5 | Faster truss strategy with similar RSS |
| `space-frame-400k` | 1138.850 | 1562.2 | Fast 3D frame assembly and memory probe |
| `plane-panel-400k` | 86160.547 | 1849.5 | Heavy triangular structural surface probe |
| `plane-quad-panel-400k` | 93180.998 | 1759.6 | Heavy structural surface probe |

`compound-core` also has a first `400k` remote smoke with
`SOLVER_PRECONDITIONER=symmetric-gauss-seidel`:

| Case | Median ms | Peak RSS MiB | Notes |
|---|---:|---:|---|
| `truss-roof-400k` | 59250.060 | 1335.3 | SGS truss leg inside compound matrix |
| `space-frame-400k` | 899.321 | 1529.2 | Fast 3D frame leg inside compound matrix |
| `heat-plane-quad-400k` | 6747.954 | 1529.2 | Thermal leg inside compound matrix |
| `compound-surface-panel-400k` | 80774.351 | 1776.6 | Heavy surface leg inside compound matrix |

`thermal-structural` now has usable large-scale single-case thermal surface
probes. The original long-run blocker was `thermal-bar-400k`; the solver now
uses a chain-specific tridiagonal fast path, and the remote single-case probe
completes in about `190.527 ms` with roughly `512.6 MiB` peak RSS. The next
blocker was `thermal-plane-triangle`: validation and precompute previously
rebuilt full node views per element. Those paths now reuse one converted node
view per solve, so `thermal-plane-triangle-100k` completes in about
`10447.681 ms`, and `thermal-plane-triangle-400k` completes in about
`97332.506 ms` with Jacobi or `64778.082 ms` with symmetric Gauss-Seidel.
The matching quad surface path also works with `auto`: `thermal-plane-quad-100k`
completes in about `7562.447 ms`, and `thermal-plane-quad-400k` completes in
about `64423.066 ms` with roughly `1625.6 MiB` peak RSS.
The full `thermal-structural-400k` matrix now completes as a remote smoke with
`SOLVER_PRECONDITIONER=auto`: all nine cases pass with about `121496.498 ms`
summed median time and roughly `1625.8 MiB` peak RSS. The two surface cases
remain the dominant cost; the remaining frame/truss fixtures are effectively
sanity probes at this scale tier. Use `make benchmark-profile-report` with the
original `OUTPUT_SLUG` to rebuild the Markdown summary from copied JSON without
touching the remote host.

First full `thermal-structural-400k` remote smoke:

| Case | Median ms | Peak RSS MiB | Notes |
|---|---:|---:|---|
| `thermal-bar-400k` | 189.547 | 512.8 | Chain-specific tridiagonal fast path |
| `thermal-truss-2d-400k` | 0.019 | 512.8 | Small coupled fixture sanity check |
| `thermal-truss-3d-400k` | 0.014 | 512.8 | Small coupled fixture sanity check |
| `thermal-plane-triangle-400k` | 64092.147 | 1512.7 | SGS selected by `auto`; dominant surface solve |
| `thermal-plane-quad-400k` | 57214.733 | 1625.8 | SGS selected by `auto`; dominant surface solve |
| `frame-2d-400k` | 0.016 | 1625.8 | Small static frame fixture sanity check |
| `frame-3d-400k` | 0.010 | 1625.8 | Small static frame fixture sanity check |
| `thermal-frame-2d-400k` | 0.005 | 1625.8 | Small coupled frame fixture sanity check |
| `thermal-frame-3d-400k` | 0.007 | 1625.8 | Small coupled frame fixture sanity check |

SPD profiles can compare solver preconditioners by setting
`SOLVER_PRECONDITIONER=all`. The benchmark emits separate
`#jacobi` and `#symmetric-gauss-seidel` result rows, including iteration count
and residual norm. Use `SOLVER_PRECONDITIONER=jacobi` or
`SOLVER_PRECONDITIONER=symmetric-gauss-seidel` for a single-strategy smoke.
Use `SOLVER_PRECONDITIONER=auto` when you want the benchmark to keep Jacobi for
general cases but select symmetric Gauss-Seidel for large thermal-plane
triangle/quad probes.

Current regression-gate default:

- cases with baseline median below `5.0 ms` are still reported, but are not
  treated as hard regression failures by default
- `make benchmark-standard-compare` uses the same default gate thresholds as
  `make benchmark-compare`: `+25%` median, `+20%` peak RSS, and `5.0 ms`
  minimum baseline median

That keeps tiny micro-benchmarks such as the axial bar cases visible without
letting timer noise dominate the gate.
