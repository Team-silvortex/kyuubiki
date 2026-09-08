# PCG Vector Cancellation: 2026-09-08

## Verdict and scope

**Installed PCG vector cancellation: PASS, 11/11 new fault scenarios.**
Qualification: `pcg-vector-installed-thermal-recovery`. This extends the
[matrix-product/residual study](matvec-residual-research-20260908.md) to PCG's
internal vector passes. Qualification remains
`synthetic_reference_not_material_certification`.

| Gate | Executed result |
| --- | --- |
| Initial tests against old PCG paths | 0 pass / 9 expected failures for ignored cancellation |
| Complete final solver unit/integration run | 714 pass across 153 test targets, 0 failures, 2 explicit benchmarks ignored |
| Final solver units in that total | 244 pass, including 16 new functional tests |
| Optimized release solver units | 244 pass, 0 failures; benchmarks selected separately |
| Optimized paired matrix/vector benchmarks | 2/2 explicit pass, run sequentially |
| Final CLI units | 151 pass |
| Cargo-built and Installer-installed cancellation / orphan / lifecycle / shutdown | 35 / 7 / 5 / 8 pass in each final run |
| Cargo-built TaskIR solve, tamper rejection and next healthy TCP execution | 1 pass |
| Installed mixed-workflow baseline | Pass against preceding physical baseline |
| Fresh public-SDK thermal and thermal-stress reference matrix | 302/302 pass, 0 failures |
| Exact original-case readback before and after idle restart | 302/302 at both stages |
| macOS ARM64 solver/CLI test compilation, Rust 1.88 | Pass |

The ordinary solver run deliberately ignores two performance tests; both are
explicitly executed with `--release --ignored --nocapture --test-threads=1`.
The four installed live executables contain 55 passes, including shared startup
ownership tests. Their 11 new scenarios are nine cancellation boundaries and two
transport-loss boundaries, not eleven different material models. CLI units use
a Cargo test executable; installed live tests select the installed Agent binary.

## Numerical contract and regression

The following PCG passes now return `Result` and propagate cancellation:
`pcg_rhs_scale`, `pcg_rhs_normalize`, `pcg_direction_copy`, `pcg_dot`, `pcg_norm`,
`pcg_vector_update`, `pcg_residual_update`, `pcg_direction_update` and
`pcg_solution_scale`. Each polls at entry, every 1,024 completed elements and
the final short block, including entry for empty input. Counters reset for each
invocation. Norm counters include visited zero-valued elements. These are not
iteration counts, percentages, saved numerical state or a latency guarantee.

Block boundaries retain the same accumulator, operation order and convergence
criteria. No regrouped dot products, changed tolerances, or parallel reductions
are introduced. Final rescaling must succeed before the solution is returned.
Cancelled mutable buffers can contain a prefix of scratch values; that is not
a completed solution or a checkpoint that may be resumed.

Nine tests compiled against the old paths and failed because the raw solve
returned success without reaching the requested cancellation boundary. Final
tests assert the raw operation returns an error inside the control scope, not
only the outer scope's success filter. They cover all nine stages, lengths
0/1/63/1023/1024/1025/2049, terminal blocks, pending tokens, partial scratch,
and unchanged finite arithmetic/non-finite classification. Independent old-loop
references check vector values, reductions and the stable norm recurrence.

Jacobi, symmetric Gauss-Seidel and IC(0) each exercise cancellation followed by
re-solving at sizes 49 and 2049. Solutions, residual norms and iteration counts
match an uninterrupted run. Small systems must not convert cancellation into a
dense fallback success; reusable prepared systems must not turn cancellation
into a regularized retry. A zero RHS still reaches the scale check before its
zero-solution shortcut. Separate sparse-system scaling helpers are not covered
by these PCG checks.

The installed heat fixture has 40 by 40 Q4 cells and 1,681 physical nodes,
with 1,599 free temperature DOFs. All nine holds are observed at exactly 1,024
completed elements of the selected vector pass. Cancellation returns code
`cancelled`, the matching stage/count, no result, and `resumable=false`.
The marker still exists when active capacity reaches zero. Only then is it
removed and the same job resubmitted, verifying every nodal temperature against
the linear analytical field at the unchanged 1e-6 K tolerance.

Additional tests disconnect the request during `pcg_vector_update` and
`pcg_solution_scale`. The original execution fails at the exact held boundary
and releases capacity without marker removal. The healthy replay computes the
complete field. The final-scaling case prevents an already-converged but
incompletely rescaled vector from becoming a successful result.

## Paired performance

The optimized microbenchmark exercises the actual private helpers on vectors of
**1,000,000 elements**, not a million-node FEM solve. Nine measured samples rotate
old-loop reference, unscoped and controlled variants after three warmup samples;
each sample runs 20 passes. Inputs are black-boxed; workspace setup/cloning and
output checks are outside timing. Outputs are compared with the sequential reference.
The control scope encloses the repeated passes, as for a normal PCG invocation.

The initial explicit maximum loop was not accepted: RHS scale cost rose from
0.298715 ms to 0.806343 ms under control, about 2.70 times the paired baseline.
Restoring the original iterator `fold`, with the carried accumulator as each
block's initial value, removed that regression without changing polling points.
The first benchmark's residual reference was also corrected to use a local
accumulator, like the old implementation, instead of repeatedly writing a field.
That initial residual timing is not used as a performance comparison.

Final repeat, median milliseconds/pass:

| Kernel | Old-loop baseline | Unscoped | Controlled |
| --- | ---: | ---: | ---: |
| RHS scale | 0.278939 | 0.301441 | 0.297652 |
| RHS normalize | 0.721749 | 0.727105 | 0.745729 |
| Direction copy | 0.484824 | 0.554202 | 0.539385 |
| Dot | 0.724734 | 0.734452 | 0.740537 |
| Stable norm | 1.016678 | 1.021738 | 1.019580 |
| Solution/residual update | 1.303888 | 1.329108 | 1.329264 |
| Exact residual vector update | 1.567007 | 1.133940 | 1.141124 |
| Direction update | 0.822593 | 0.554805 | 0.602664 |
| Final solution scale | 0.169565 | 0.178283 | 0.185098 |

The preceding final-implementation run measured RHS scale at
0.337036/0.355121/0.364071 ms and dot at 0.794777/0.838818/0.880243 ms.
The shared host and code-generation context affect these microbenchmarks;
faster isolated loops do not imply a corresponding whole-solver speedup.
**No global overhead bound** is claimed. The largest positive vector overhead
in the final repeat is direction copy, about 11.25%; dot and fused solution/
residual update are about 2.18% and 1.95%. The retained short-row matrix-product
benchmark also passes at 2.741698/2.921990/2.918404 ms for one million matrix rows
and 2,999,998 nonzeros. Installed functional tests use a debug-profile Agent,
not the optimized benchmark executable.

## Installed research and provenance

The official Rust Headless SDK submits fresh work to an Installer-installed
isolated runtime with distributed dispatch, SQLite and transitional worker
adapters disabled. Independent baseline job `fc9aebcacdafb90f` completes its
seven-node graph and matches the preceding study's full physical output.
Its 153 physical nodes and 128 Q4 cells are a small mixed fixture, not the
large PCG heat fixture used by the fault tests. No Orchestra process-loss fault
at a PCG vector stage is claimed in this round.

Baseline maximum temperature/displacement/flux errors are 6.310e-12 K,
6.806e-17 m and 7.687e-9 W/m2, under unchanged tolerances 1e-7 K, 1e-9 m
and 7.921e-5 W/m2. The fresh matrix contains 14 layered and 288 triangle/quad
variations, all passing physical checks, output-manifest validation and readback.
These synthetic references do not certify real materials or arbitrary solvers.

Before idle restart, Agent instance `agent-instance-7-1788875292672-1` records
606 started/completed executions, zero failures and zero active executions.
Both owned processes stop with exit 0 and no OOM, then restart. All 302 original
results pass exact read-only verification; the independent baseline result is
byte-identical. Fresh Agent `agent-instance-7-1788875659964-1` records zero
started/completed/failed/active executions during readback. Recovery does not
silently substitute recomputation. The phased baseline verifier also checks
600 ms terminal stability, not unlimited exactly-once behavior.

Base revision is `9fd06aa8b7893d3bdc63e8503d512ab93239ef38` plus the retained
16-file Rust source/test overlay, including the preceding uncommitted product
work. All 16 local/remote digests match. Installer seals and installs candidate
3.1.5; actual Cargo, reused Orchestra and SDK metadata remain 3.0.0. The initial
pre-optimization installation is not used for qualification; the final candidate
uses a separate managed configuration root, preserving installation immutability.
The unused initial installation copy is removed after final qualification; its
installation records and intermediate source/performance evidence are retained.
No version bump, commit, GUI repackaging or production promotion occurs.

Installed TCP fault tests run owned non-root Agent processes on the Linux host.
SDK research uses two non-root uid 1001 containers with read-only root/payload,
separate writable state, init, no capabilities, no-new-privileges, four CPUs each,
and 4 GiB / 2 GiB limits. HTTP/TCP are loopback-only 6480/6481. The native build
uses Rust 1.95.0 and the reused release Elixir 1.19.5 / OTP 28.5.0.2.
Normal SDK research has all fault-injection variables unset. It is not
multi-host, HA, installed macOS/Windows, or source-tree Mix qualification.

Evidence stays under managed server state
`research-runs/vector-research-20260908/`: red/green/final source overlays,
source digests, installation records, benchmark logs, installed live checkpoints,
full research arrays, SQLite state, readback and process exits. Final installation
is under `config-final/`; no server configuration, credential or bulk result is
added to Git. Both owned containers are stopped and removed. The original
services and unrelated containers are not restarted or modified.

| Retained artifact | SHA-256 |
| --- | --- |
| Final installed Agent | `795518a6ab8b5100640b73505cc08375ce44bfc5867a9449451a0bd43cc0177d` |
| Final payload manifest | `59932d43e5336ae229d6df959b6e02c7fc7e615d6fba7ca6e69d68f4428c076f` |
| Final source/test overlay | `e528df1b88367a63da73aca8c62a9e634618b9c49b168e0f61da8bb2a65d8ab7` |
| Fresh 302-case report | `22f3e64f0d2b60d8a72fac1162ccee1830948ed55d6a01a180d5023d82664cb5` |

Digests identify bytes, not signatures or certificates. Reproduce with
`cargo test -p kyuubiki-solver --tests` and
`cargo test -p kyuubiki-solver --release --lib paired_ -- --ignored --nocapture --test-threads=1`.
Select the installed Agent using `KYUUBIKI_TEST_AGENT_BINARY`; retain owned
test evidence with `KYUUBIKI_TEST_AGENT_EVIDENCE_DIR`. See the
[HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation).
Only explicitly owned isolated work may be subjected to fault injection.

Remaining gaps include sparse-system scaling/validation, allocation, physical
postprocessing, dense/preconditioner row interiors, other assembly paths,
custom constraints, nonlinear outer loops, analytical shortcuts, external
workers and child threads. Network blackholes, power loss, database corruption,
external-solver cross-validation and material calibration remain separate work.
Scoped tensor evidence does not change overall release maturity.

The 25-page HTML book, documentation inventory, Rust workspace formatting,
tensor structure/self-test and organization audit/self-test pass. Source/document
limits remain 800/2000 with zero tracked debt. The global tensor still reports
`daji status=blocked`; these scoped qualifications do not override it.
