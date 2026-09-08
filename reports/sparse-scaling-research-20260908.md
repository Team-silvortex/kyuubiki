# Sparse Scaling and Validation Cancellation: 2026-09-08

## Verdict and scope

**Installed sparse scaling/validation cancellation: PASS, 8/8 new fault scenarios.**
Qualification: `sparse-scaling-installed-thermal-recovery`. This extends the
[PCG vector study](pcg-vector-research-20260908.md) to shared sparse-system finite
checks, scaling, final unscaling and regularized fallback preparation.
Qualification remains `synthetic_reference_not_material_certification`.

| Gate | Executed result |
| --- | --- |
| Initial tests against old sparse helpers | 0 pass / 13 expected failures for missing cancellation |
| Complete final solver unit/integration run | 735 pass across 153 test targets, 0 failures, 3 explicit benchmarks ignored |
| Final solver units in that total | 265 pass, including 21 new functional tests |
| Optimized release solver units | 265 pass, 0 failures; benchmarks selected separately |
| Optimized paired product/vector/scaling benchmarks | 3/3 explicit pass, run sequentially |
| Final CLI units | 151 pass |
| Cargo-built and Installer-installed cancellation / orphan / lifecycle / shutdown | 43 / 7 / 5 / 8 pass in each run |
| Cargo-built TaskIR solve, tamper rejection and next healthy TCP execution | 1 pass |
| Installed mixed-workflow baseline | Pass against preceding physical baseline |
| Fresh public-SDK thermal and thermal-stress reference matrix | 302/302 pass, 0 failures |
| Exact original-case readback before and after idle restart | 302/302 at both stages |
| macOS ARM64 solver/CLI test compilation, Rust 1.88 | Pass |

The four installed live executables contain 63 passes, including shared startup
ownership tests. The eight new scenarios are six cancellation boundaries and
two transport-loss boundaries, not eight different material models. CLI units
use a Cargo test executable; installed live tests select the installed Agent.
Fallback and long-row cancellation are qualified at the native numerical-test
layer, not by the ordinary heat fixture's installed RPC calls.

## Numerical contract

Thirteen diagnostic stages are appended without renumbering existing stages:

| Stage | Progress unit and boundary |
| --- | --- |
| `sparse_validate_rhs` | 1,024 RHS elements |
| `sparse_validate_matrix` | 64 matrix rows, including empty rows |
| `sparse_validate_matrix_row` | 1,024 entries within a row longer than 1,024 entries |
| `sparse_diagonal_scale`, `sparse_diagonal_magnitude` | 64 matrix rows |
| `sparse_rhs_scale`, `sparse_solution_unscale` | 1,024 vector elements |
| `sparse_capacity_scan` | 1,024 rows of the scaling/regularization capacity hint |
| `sparse_matrix_scale`, `sparse_regularize_copy` | 64 matrix rows |
| `sparse_matrix_scale_row`, `sparse_regularize_copy_row` | 1,024 entries within a wide row |
| `sparse_regularize_diagonal` | 64 matrix rows |

Each listed traversal polls at entry and its final short block. Counts reset
per pass or wide row; they are not iterations, percentages, saved numerical
state or a wall-clock cancellation bound. Finite validation keeps RHS-before-
matrix error precedence and the original error strings. Its bounded boolean
reduction may inspect the remainder of a block after an invalid value; it does
not change an arithmetic reduction or report an offending index.

Diagonal scaling uses SparseMatrix's existing binary lookup on sorted, unique
columns. Missing and zero diagonals retain the old behavior. Scaled diagonal
averaging, vector/matrix multiplication and regularization preserve sequential
arithmetic, sparse entries and solver tolerances. The sum starts from the same
negative-zero identity as the old iterator sum. Row buffers are now allocated
along the checked traversal instead of preallocating all rows uninterrupted.

Ordinary and reusable prepared solvers propagate helper errors. Cancellation
cannot become a regularized retry, a partially scaled matrix or an accepted
partially unscaled profile. Iteration counts, nonzero counts, residual norms
and stage metadata are retained on success. No numerical checkpoint/resume
format is introduced by this change.

The 13 red tests assert the raw public solve returns an error inside its control
scope, not merely the outer scope's cancellation filter. Old normal paths
returned success; old forced fallback paths returned an ordinary preconditioner
error instead of the selected cancellation. Final tests cover all 13 stages,
lengths 0/1/63/64/65/1023/1024/1025/2049, empty rows, terminal blocks, negative-
definite fallback and 2,049-entry rows. Independent old-loop references compare
finite values bitwise and non-finite classification, including absent/zero
diagonals and epsilon 0/0.125/-4/1e-300. NaN and both infinities are injected at
vector/row boundaries; cancellation before a later invalid block is checked.

A reference-fixture gap was corrected before final qualification: clearing row
zero had accidentally removed its only wide row. The final fixture clears the
last row instead and preserves wide-row arithmetic coverage. Separate tests
check binary diagonal lookup at the beginning, middle and end of wide rows.
Prepared-factor reuse and Jacobi/SGS/IC(0) cancellation followed by a healthy
solve retain solution/residual bits, iterations and nonzero counts.

## Installed Agent and research

The installed fault fixture contains 40 by 40 Q4 cells, 1,681 physical nodes
and 1,599 free temperature DOFs. Six cancellation holds are observed at exactly
64 rows for matrix validation/diagonal scale/diagonal magnitude and 1,024
elements for RHS validation/RHS scale/final unscale. Each returns code
`cancelled`, its exact stage/count, no result and `resumable=false`. Active
capacity reaches zero while the marker still exists; only then is the marker
removed and the same job replayed. Every nodal temperature must match the
linear analytical field at the unchanged 1e-6 K tolerance.

Two additional cases disconnect TCP at RHS validation and final unscaling.
Watchdog failures identify the exact stage and 1,024 completed steps; active
capacity is zero before marker removal. Both healthy replays pass. In
particular, convergence does not authorize publishing an incomplete final
unscaling prefix. Normal SDK research leaves all fault controls unset.

The official Rust Headless SDK submits fresh work to an Installer-installed
isolated distributed runtime with SQLite and transitional worker adapters
disabled. Baseline job `9fff5f75827821d3` completes its seven-node graph and
matches the preceding verified physical output. Its 153 physical nodes and
128 Q4 cells are a small mixed fixture, not the installed cancellation fixture.
Maximum temperature/displacement/flux errors are 6.310e-12 K, 6.806e-17 m and
7.687e-9 W/m2, under unchanged tolerances 1e-7 K, 1e-9 m and 7.921e-5 W/m2.
The fresh matrix includes 14 layered and 288 triangle/quad variations, with
physical checks, output-manifest verification and readback all passing.

Before idle restart, Agent `agent-instance-7-1788880709273-1` records 606
started/completed executions, zero failures and zero active executions. Both
owned services stop with exit 0 and no OOM, then restart. All 302 original cases
pass exact read-only verification, and the independent baseline result file is
byte-identical. New Agent `agent-instance-7-1788881425933-1` records zero
started/completed/failed/active executions through readback, so recovery does
not substitute recomputation. The baseline verifier also checks 600 ms terminal
stability. This is not an Orchestra process-loss fault or an HA qualification.

## Paired performance and provenance

The optimized benchmark invokes actual private helpers. Three warmup samples
precede nine measured samples rotating old-loop, unscoped and controlled modes;
each measured sample contains eight passes. Inputs are black-boxed and outputs
checked against old sequential references. Input/factor setup and output checks
are outside timing; output allocations and replacement of previous pass outputs
are inside timing where applicable. The control scope encloses repeated passes.

Final repeat, median milliseconds/pass:

| Kernel | Actual processed size | Old loop | Unscoped | Controlled |
| --- | --- | ---: | ---: | ---: |
| RHS finite validation | 1M vector elements | 0.218025 | 0.185460 | 0.184837 |
| Matrix finite validation | 1M rows / 3M nonzeros | 3.374840 | 3.753519 | 3.771081 |
| Diagonal scale | 1M rows / 3M nonzeros | 5.459758 | 5.964235 | 5.998151 |
| Diagonal magnitude | 1M rows / 3M nonzeros | 3.873566 | 4.130990 | 4.141205 |
| RHS scale | 1M vector elements | 1.420838 | 1.429430 | 1.431623 |
| Matrix scale | 100k rows / 300k nonzeros | 3.361874 | 3.256156 | 3.272957 |
| Regularize | 100k rows / 300k nonzeros | 3.810227 | 3.406581 | 3.448800 |

The initial finite-RHS implementation regressed from 0.211300 to 0.442974 ms,
about 2.10 times its paired reference. A bounded non-short-circuit boolean fold
removed that regression; short matrix rows retain their direct finite-check
fast path. The final preceding repeat agrees in direction, with RHS validation
at 0.219549/0.178477/0.177163 ms and matrix validation at
3.404917/3.756224/3.800210 ms. **No global overhead bound** is claimed.
The final scaling suite's largest positive controlled overhead is matrix
validation, about 11.74%; diagonal scaling is about 9.86%. Faster isolated
copy/scaling paths do not imply an equivalent end-to-end solver speedup.

The retained paired product benchmark passes at 2.446897/2.594112/2.473173 ms
for 1M rows and 2,999,998 nonzeros; all nine preceding PCG vector kernels also
pass. These are release-mode kernel measurements, not million-node physical
solves, and installed functional tests use a debug-profile Agent.

Base revision is `7dbb6aac05544d74cd1f5a676b3d8296ca926cfe` plus the retained
10-file Rust source/test overlay. Every overlay file matches its remote digest.
An additional manifest verifies all 1,247 tracked native-workspace files;
the four new untracked test/helper files are covered by the explicit overlay
manifest, giving 1,251 unique verified native files. SDK study definition is
`245beec6daf0c4647ecaba7f4e5606a9fa17fa325a481e12e34773d7bffe218c`.
Installer seals and installs only the final candidate under label 3.1.6;
actual Cargo, reused Orchestra and SDK metadata remain 3.0.0. No version bump,
commit, GUI repackaging or production promotion occurs.

Installed live faults use owned non-root Linux host Agent processes. SDK
research uses two uid 1001 containers with read-only root/payload, separate
writable state, init, no capabilities, no-new-privileges, four CPUs each, and
4 GiB / 2 GiB memory limits. HTTP/TCP bind only loopback 6490/6491. The native
build uses Rust 1.95.0; the reused runtime is Elixir 1.19.5 / OTP 28.5.0.2.
This is not multi-host, installed macOS/Windows or source-tree Mix qualification.

Evidence stays under managed server state `research-runs/scaling-research-20260908/`:
red/green/final source overlays, full native and overlay digests, install records,
release benchmarks, installed live receipts, matrix results, SQLite state,
readback and process exits. Both owned containers are stopped and removed with
exit 0 and no OOM; original services and unrelated containers are unchanged.
No server configuration, credential, installation payload or bulk result enters Git.

| Retained artifact | SHA-256 |
| --- | --- |
| Final installed Agent | `93dc0cf3ad7642cdde45bfc1870fdee724b4f5e557367cffcb567d8245edd84b` |
| Final payload manifest | `ac6e83f51318ac855ea0a8c8964420de23653000e742c8ec4a15aea6870b7a73` |
| Final source/test overlay | `030ed1f471f17ae7bc5cac376e7f1a745e6c6ae068129e574b4efd2efc944ced` |
| Fresh 302-case report | `3d1bba2e74971006f848c21f4bbfdd77127e2a24868df18f415da40fa82d7021` |

Digests identify bytes, not certificates. Reproduce numerical gates with
`cargo test -p kyuubiki-solver --tests` and the paired suite with
`cargo test -p kyuubiki-solver --release --lib paired_ -- --ignored --nocapture --test-threads=1`.
Select an installed Agent with `KYUUBIKI_TEST_AGENT_BINARY` and retain owned
live evidence using `KYUUBIKI_TEST_AGENT_EVIDENCE_DIR`. See the
[HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation).
Only explicitly owned isolated work may be subjected to fault injection.

Remaining gaps include individual allocations, Vec row insert/remove interiors,
other capacity scans, input parsing/structural checks, physical postprocessing,
dense/preconditioner row interiors, other assembly paths, nonlinear outer loops,
analytical shortcuts, custom constraints, external workers and child threads.
Power loss, network blackholes, database corruption, external-solver validation
and material calibration remain separate qualifications. Synthetic checks do
not certify real materials or arbitrary solver workloads; scoped tensor evidence
does not override the overall release maturity gate.

The 25-page HTML book, documentation inventory, Rust workspace formatting,
tensor structure/self-test and organization audit/self-test pass. Source/document
limits remain 800/2000 with zero tracked debt. The global tensor still reports
`daji status=blocked`; these scoped qualifications do not override it.
