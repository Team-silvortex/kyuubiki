# Matrix Products and Residual Acceptance: 2026-09-08

## Verdict and scope

**Installed residual cancellation: PASS, 3/3 expected research outcomes.**
Qualification: `mixed-thermal-matvec-residual-recovery`. This follows the
[constraint/preconditioner study](constraint-preconditioner-research-20260908.md)
with cancellation inside matrix products and residual acceptance, including
wide-row interiors. Qualification remains
`synthetic_reference_not_material_certification`.

| Gate | Executed result |
| --- | --- |
| Initial tests against old numerical paths | 0 pass / 6 expected failures for ignored cancellation |
| Complete final solver unit/integration run | 698 pass across 153 test targets, 0 failures, 1 explicit benchmark ignored |
| Final solver units in that total | 228 pass, including 13 new functional tests |
| Optimized release solver units | 228 pass, 0 failures; benchmark selected separately |
| Optimized paired matrix-product benchmark | Explicit pass, including two final runs |
| CLI units, separate 16-thread and 32-thread runs | 151 pass in each |
| Cargo-built and Installer-installed cancellation / orphan / lifecycle / shutdown | 24 / 7 / 5 / 8 pass in each run |
| Cargo-built TaskIR solve, tamper rejection and next healthy TCP execution | 1 pass |
| Installed baseline / permitted replay / checkpoint-required rejection | 3/3 expected outcomes |
| Fresh public-SDK thermal and thermal-stress reference matrix | 302/302 pass, 0 failures |
| Exact original-case readback before and after idle restart | 302/302 at both stages |
| macOS ARM64 solver/CLI test compilation, Rust 1.88 | Pass |

The benchmark is intentionally ignored by ordinary test discovery and explicitly
run with `--release --ignored --nocapture`; it is not silently counted as a
functional pass. The four live executables repeat the shared startup-ownership
regression. Their 44 passes are not 44 distinct physical models. Installed tests
select the installed Agent binary; CLI units still use a Cargo test executable.
The 302-case matrix contains 14 layered cases and 288 triangle/quad variations.
None of these counts is a global reliability percentage or material certification.

## Changes and performance

Compressed sparse products previously had no fallible return or internal safe
points. They now return `Result`, with cancellation propagated through PCG,
residual recomputation, modal products and buckling subspace/refinement paths.
Cancellation is not converted into a successful vector or ordinary fallback.

Shared residual vectors, stable residual norms and relative residual validation
also propagate cancellation. Dense refinement and reusable tridiagonal/dense/PCG
backends must finish acceptance checks before returning a solution. The stable
norm uses the same scaling recurrence and non-finite behavior without allocating
an intermediate residual vector. Row sums retain their original entry order and
accumulator; chunking does not regroup floating-point reductions.

`sparse_matvec` and `sparse_residual` poll at entry, every 64 rows and completion.
`residual_validate` counts rows cumulatively through equation-scale and worst-row
passes. Rows longer than 1,024 entries additionally poll at entry, 1,024-entry
boundaries and completion: `sparse_matvec_row`, `sparse_residual_row`, and
`residual_validate_row`. Wide-row counters reset per row, do not identify a row
number, and are not whole-job progress or saved numerical state.

Six initial tests compiled against the previous numerical paths and all failed
for ignored cancellation. Final tests assert the raw operation returns an error
inside its control scope. They cover short/empty systems, both relative-validation
passes, wide-row partial output, repeated prepared solves, and modal/buckling
propagation. Products and residual vectors are compared bitwise with the old
entry-order reference; subsequent healthy solves remain identical. Scaling and
unscaled compression both validate the cached maximum row width around the
1,024-entry boundary. Earlier complete rows may remain in scratch after an error;
that scratch is not a valid completed vector.

The first implementation exposed a substantial slowdown, so it was not accepted
as-is. Short rows now use a direct kernel selected from immutable maximum-row
metadata gathered during compression, with no extra full-matrix scan. Row-level
polling is outside 64-row blocks; the wide-row path retains its internal checks.
`checkpoint_chunk` is inlined, avoiding a function call on every short step.
No polling boundary, numerical tolerance or physical gate was relaxed.

The paired release microbenchmark uses **1,000,000 matrix rows and 2,999,998
nonzeros**, three entries per interior row. It is a synthetic matrix-product
kernel benchmark, not a million-node FEM solve. Nine samples rotate the old-loop,
unscoped and controlled variants, with 20 products per sample, `black_box`, warmup
and exact output comparison. Each value below is the median milliseconds/product.

| Implementation/run | Old-loop baseline | Unscoped | Controlled |
| --- | ---: | ---: | ---: |
| Initial generic per-row wrapper | 2.711392 | 6.866585 | 6.964107 |
| Inline/short-row path | 2.844167 | 4.051676 | 4.090998 |
| 64-row batching | 2.639999 | 3.689822 | 3.716950 |
| Final row-width classification | 2.533062 | 2.687395 | 2.724195 |
| Final repeat | 2.989610 | 3.045932 | 3.059041 |

Final controlled overhead is about 7.55% and 2.32% against the respective paired
old-loop baselines, versus about 157% in the initial experiment. This does not
mean arbitrary simulations have 2%-8% overhead. The host is shared, samples vary,
and only this short-row kernel shape is timed. Wide-row cancellation is tested
functionally, not performance-qualified here. **No global overhead bound** or
cancellation-latency bound is claimed. Installed functional tests use the
Cargo-built debug-profile Agent; optimized kernel timings are a separate result.

`linear_algebra.rs` shrinks from 607 to 499 lines. Products, residual checks and
their tests live in focused files, all below the 800-line source limit.

## Installed research and provenance

| Submission | Job id | Expected outcome |
| --- | --- | --- |
| `baseline` | `9efa6af555b9edd7` | Generation 1 completes and matches the preceding physical baseline |
| `orchestra-replay` | `d74a6f2ca93131ef` | Original job completes at generation/attempt 2 with one terminal commit |
| `orchestra-blocked` | `128e45c50a1af805` | Generation/attempt 1 stays recovery-blocked without successful structure output |

The public Rust Headless SDK drives an Installer-installed isolated runtime,
with distributed dispatch and transitional worker adapters disabled. The
seven-node graph uses the existing 153-node, 128-Q4-cell mixed high-contrast
fixture. Both Orchestra SIGKILLs occur in `solve_thermal_plane_quad_2d` at
`residual_validate` after exactly 64 completed validation rows. Both exit 137
without OOM. A generic active/held label is not sufficient fault evidence.

Original replay request `2c7f4d1ed74b04cf` reports
`solver cancelled at residual_validate after 64 steps` while its marker still
exists. Replacement `d624f21bda745a63` reaches the same stage on unchanged Agent
instance `agent-instance-7-1788868149199-1`, with one active execution. Agent
execution generations 4 and 6 are not workflow attempt numbers. Marker release
then permits the replacement to complete full residual acceptance.

Checkpoint-required request `3b2397f9562db432` cancels at the same boundary.
Before marker removal, active count is zero, total-started remains 8, and
total-completed remains 6. No replacement calculation is hidden behind the
blocked label. Removing the marker does not change the blocked outcome. This
is safe rejection, not numerical acceptance or saved-state resume.

Full physical arrays and bridged models match the uninterrupted baseline.
Maximum temperature/displacement/flux errors remain 6.310e-12 K, 6.806e-17 m
and 7.687e-9 W/m2 under unchanged tolerances 1e-7 K, 1e-9 m and 7.921e-5 W/m2.
The phased verifier checks 600 ms terminal stability, not unlimited exactly-once
behavior. All 302 fresh cases pass; their original results pass exact readback
before and after both idle processes stop with exit 0 and restart. All three
phased result files remain byte-identical. The fresh Agent starts zero executions
during readback: validation does not silently substitute recomputation.

Base revision is `9fd06aa8b7893d3bdc63e8503d512ab93239ef38` plus a retained
12-file Rust source/test overlay; all 12 local/remote digests match. Installer
seals and installs candidate **3.1.5** in an isolated managed root. Actual Cargo
and reused Orchestra release metadata remain **3.0.0**. No repository version
bump, commit, GUI repackaging, system installation promotion or production
process replacement occurs in this round.

One Linux host runs non-root uid 1001 containers with read-only root/payload,
separate writable state, init, no capabilities, no-new-privileges, four CPUs each,
and 4 GiB / 2 GiB limits. HTTP/TCP are loopback-only 6470/6471; storage is SQLite.
The Agent is built with Rust 1.95.0; the reused release uses Elixir 1.19.5 /
OTP 28.5.0.2. This is not HA/multi-host qualification. The old host Mix mismatch
identified by the preceding study is not changed or requalified here.

Artifacts stay in managed server state under
`research-runs/product-research-20260908/`. `evidence/verification/` retains red
failures, intermediate slow benchmarks, final runs, source digests/overlays,
installation records and process exits. Full submissions, installed live
observations, matrix/readback results and SQLite state remain remote, not in Git.
Credentials and host configuration are not added to the repository. Earlier
studies are preserved. Both owned containers are stopped and removed; original
PIDs 3461/3462/3463 retain their August 31 start times and unrelated services
remain running. Managed payloads, database and evidence are retained.

| Retained artifact | SHA-256 |
| --- | --- |
| Installed Agent | `c7aa05a429141eb67791c7c47c88c5a64602840a21c799fec3b2988a2e0d4d05` |
| Installed payload manifest | `d1cbf5b10cdfc2c625dfb72797cb0a444108d4e9f9a33b52d39c40c38304c09e` |
| Final source/test overlay | `54ca0f82697e4569916e42b8295bcb0b47a4501a86574ee97358f4d71c795926` |
| Fresh 302-case report | `4d482f90f297a7b223f8d48bc5a342da199152d0ebcdd61127b5d9daaf451042` |

Digests identify bytes, not signatures or material certificates. Reproduction:
run `cargo test -p kyuubiki-solver --tests`; explicitly select
`cargo test -p kyuubiki-solver --release --lib paired_matvec_control_overhead -- --ignored --nocapture`.
Use the [HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [numerical control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation)
for installed faults. `KYUUBIKI_TEST_AGENT_BINARY` selects the installed Agent;
`KYUUBIKI_TEST_AGENT_EVIDENCE_DIR` retains test-owned evidence. Faults may target
only explicitly owned isolated work, never another user's running job.

Remaining gaps include allocation, input validation, physical postprocessing,
PCG vector updates/dot products, dense/preconditioner row interiors, custom
constraints, other assembly paths, analytical shortcuts, nonlinear outer loops,
external workers and child threads. Network blackholes, power loss, database
corruption, external-solver cross-validation, material calibration and installed
Windows/macOS live cancellation remain separate work. The scoped tensor claims
do not override overall release maturity or certify real materials.

The 25-page HTML book, documentation inventory, Rust workspace formatting,
tensor self-test/structure and organization audit/self-test pass. Source/document
limits remain 800/2000 with zero tracked debt. The global tensor still reports
`daji status=blocked`; scoped execution and kernel evidence do not override it.
