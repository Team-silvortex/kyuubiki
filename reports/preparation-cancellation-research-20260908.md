# Numerical Preparation Cancellation: 2026-09-08

## Verdict and scope

**Installed preparation cancellation: PASS, 3/3 expected research outcomes.**
Qualification: `mixed-thermal-preparation-cancellation-recovery`. This extends
the [numerical-kernel study](cooperative-solver-research-20260908.md) into actual
element preparation and matrix setup, not merely a pre-execution hold.

The mixed studies and 302-case matrix use the public Rust Headless SDK against
an isolated, native Installer-installed Linux Orchestra and Agent. Dispatch is
distributed, transitional worker adapters are disabled, and no mock/local solver
fallback is requested. Qualification remains
`synthetic_reference_not_material_certification`.

| Gate | Result |
| --- | --- |
| Solver unit suite, including 13 new preparation regressions | 203 pass |
| Selected thermal, heat, modal, buckling and transient integration suites | 28 pass across 12 suites |
| CLI unit tests, 16-thread and separate 32-thread runs | 151 pass in each run |
| Cargo-built and Installer-installed numerical cancellation / orphan / lifecycle / shutdown | 15 / 7 / 5 / 8 pass in each run |
| TaskIR solve, tamper rejection and subsequent healthy TCP execution | 1 pass, Cargo-built CLI |
| Installed baseline / permitted replay / checkpoint-required rejection | 3/3 expected outcomes |
| Fresh distributed layered and triangle/quad patch cases | 302/302 pass, 0 failures |
| Original-case exact readback before and after idle process restart | 302/302 at both stages |
| macOS ARM64 CLI and integration-test compile check, Rust 1.88 | Pass |

The four installed live suites include the same shared startup-ownership
regression once per test executable; 35 passes are not 35 distinct physical
models. The reference matrix contains 14 layered cases and 288 patch variations.
Neither these counts nor three expected fault outcomes are a general reliability
percentage, independent material certifications, or a scaling benchmark.

## Defects and changes

1. Planar heat and thermal-stress triangle/quad solvers did not observe
   cancellation while precomputing or assembling elements. They now check at
   entry, every 64 completed elements, and the final short batch. A cancelled
   assembly unwinds its owned state instead of continuing to solve/export it.
2. Shared sparse compression and IC(0) construction had no error return for
   cancellation. Compression, inverse-diagonal setup, IC(0) factorization and
   transpose construction now propagate `Result` errors. Ordinary, prepared,
   modal and buckling callers propagate them. No half-built compressed matrix or
   preconditioner is returned as usable state. Arithmetic ordering, regularization
   parameters and physical tolerance gates remain unchanged.
3. The live test harness selected a free port and accepted any listener on it as
   readiness. Wider concurrent testing exposed false acceptance of an invalid
   Agent configuration and a refused connection in another test. Startup now
   serializes only port selection through readiness, checks child exit first,
   and verifies the unique test Agent id in `deployment_readiness`. Live work
   remains concurrent. A regression deliberately points a rejected child at a
   different live Agent and proves that listener cannot mask startup failure.

Sparse compression moved out of `linear_algebra.rs`, reducing it from 784 to
672 lines. Added source files and modified tests stay below the 800-line limit.
Cancellation is cooperative, not a thread-kill mechanism or saved numerical state.
The six new diagnostic names are `element_precompute`, `element_assembly`,
`sparse_compress`, `preconditioner_setup`, `ic0_factor`, and `ic0_transpose`.
The last name counts rows across count, prefix-sum and fill passes, cumulatively.

After correcting test-only field names and moved options, the initial nine-test
fixture compiled against the old numerical paths and all nine tests failed
because their preparation stages ignored cancellation. The fixed suite adds
unscaled compression, reusable preparation, all IC(0) transpose passes and a
49-element final-short-batch regression. All 13 now pass. Tests compare subsequent
solutions with uninterrupted values and analytical heat/free-expansion references.

Installed TCP cases observe exactly 64 completed preparation steps on 1,681-node
heat meshes before explicit cancellation. They cover both triangle and quad
element preparation, sparse compression and inverse-diagonal setup. Additional
cases cover transport loss, freed capacity while the marker still exists, same-job
reuse, and healthy independent jobs before/after cancellation. IC(0) construction
is covered through native solver tests, including the public Q4 heat profile on a
1,681-node physical mesh; installed heat RPC cases use their default preconditioner.
This is not installed Agent IC(0) fault qualification or a new RPC solver option.

## Installed workflow evidence

| Submission | Job id | Expected outcome |
| --- | --- | --- |
| `baseline` | `6be4e0afaa75ca62` | Generation 1 completes and matches the previous qualified physical baseline |
| `orchestra-replay` | `a58b7baa2f810f0d` | Original job completes at generation/attempt 2, with one terminal commit |
| `orchestra-blocked` | `af2ea16b2ddc13d9` | Generation/attempt 1 stays recovery-blocked, without successful structure output |

The seven-node mixed graph uses the existing 153-node, 128-Q4-cell high-contrast
fixture. Both process faults occur inside `solve_thermal_plane_quad_2d` after
exactly 64 elements have been assembled. The exact-job marker, structural method,
and `KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE=element_assembly` select the
fault point. A generic active/held label alone is not sufficient evidence.

For permitted replay, request `a18e5520cc0d7401` reports
`solver cancelled at element_assembly after 64 steps`. The marker was still
present. Replacement request `bdae418d0872c354` reaches the same preparation stage
with only one active execution. Agent instance
`agent-instance-7-1788863250764-1` stays unchanged; it is not restarted between
these requests. Marker release then allows the original job to finish.

For checkpoint-required rejection, request `2fcd64a47eb00f87` reports the same
preparation-stage cancellation. With its marker still present, active count is
zero, total-started stays 8, and total-completed stays 6. No replacement execution
is hidden behind the blocked label. Releasing the marker does not change the
blocked outcome. This is safe rejection, not numerical acceptance.

Both owned Orchestra SIGKILLs exit 137 without OOM. The positive outcomes compare
full nodal/element physical arrays and bridged models with the uninterrupted
reference. Maximum errors remain 6.310e-12 K, 6.806e-17 m and 7.687e-9 W/m2,
below unchanged gates 1e-7 K, 1e-9 m and 7.921e-5 W/m2. The phased verifier checks
600 ms of terminal stability, not an unlimited exactly-once guarantee.

The fresh 302-case matrix has zero failures. All original results pass exact
readback before and after both idle processes stop with exit 0 and restart.
The three phased result files are byte-identical after restart. The fresh Agent
reports zero started executions after readback, proving the checks did not
silently submit replacement computations. This is persistence readback, not
power-loss durability or numerical checkpoint resumption.

## Provenance and retention

Base revision is `3aab326bbaa9b162f111bd8fe9ceb4f5150f15b1` plus the retained
15-file Rust source/test overlay. All 15 local/remote source digests match.
Installer seals and installs candidate label **3.1.4** into an isolated managed
runtime root. Existing Cargo and reused Orchestra release metadata remain their
actual **3.0.0** values. No repository version bump, commit, system installation
promotion, production process replacement or UI repackaging occurs in this task.

One physical Linux host runs non-root uid 1001 containers with read-only
root/payload, separate writable state, init, no capabilities, no-new-privileges,
four CPUs each and 4 GiB / 2 GiB memory limits. HTTP/TCP are loopback-only
6450/6451; Orchestra uses SQLite. The Agent was built with Rust 1.95.0; the
reused release uses Elixir 1.19.5 / OTP 28.5.0.2. This is not an HA/multi-host test.

Large artifacts remain in managed server state under
`research-runs/preparation-research-20260908/`. `evidence/` retains submissions,
numerical-stage observations, installed live evidence and all matrix/readback
results. `evidence/verification/` retains initial failures, corrected runs,
source overlays, installation records and process exit evidence. Credentials,
host configuration, payloads and complete result sets are not added to Git.
Previous studies and failed attempts are preserved rather than overwritten.

Initial test compilation errors, the exposed listener race, an intermediate
readiness edit using the wrong descriptor field, and a mistyped Cargo target
were corrected before qualification. A separately invoked source-tree
`operator_task_live` test could not start its Mix test server (`Disconnected`).
That source-tree server path is not qualified by this report. It is not counted
as a pass or replaced by the distinct passing installed-service/TaskIR tests.
Readiness probes also briefly refused connections while owned releases started;
the research verifiers ran only after successful HTTP readiness.

| Retained artifact | SHA-256 |
| --- | --- |
| Installed Agent | `2929166c6edeb84c3f40415e6d1e11a86c49ddabcd6e78912ed26d1068493ecc` |
| Installed payload manifest | `b2c0eed4349e0e23c8a1221b69f3ce94bfd42803655326b8b29c4e558d896736` |
| Final source/test overlay | `6c7f231abf65d4a4679a5512bf07fdf265dc8c6aab539cbd7b0f89040d8a8544` |
| Fresh 302-case report | `c1f20d745e31de60762c965315072afb1c7a64a5aa3b12793e11f3e43153b2c6` |

Digests identify retained bytes, not signatures or material certificates.

The 25-page HTML book, documentation inventory, Rust formatting, tensor
self-test/structure and project organization audit pass. There is no tracked
line-limit debt. The global tensor still reports `daji status=blocked`; this
bounded qualification does not override the overall release gate.
Both owned candidate containers were stopped and removed after capture. Original
service PIDs 3461/3462/3463 retain their August 31 start times, and unrelated
containers remain running. Installation, database and evidence stay on the server.

## Reproduction and remaining boundaries

Use the [HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [numerical control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation).
Run `cargo test -p kyuubiki-solver --lib solver_preparation_tests` and
`cargo test -p kyuubiki-cli --test agent_solver_cancel_live` in the Rust workspace.
`KYUUBIKI_TEST_AGENT_BINARY` selects the installed Agent;
`KYUUBIKI_TEST_AGENT_EVIDENCE_DIR` retains test-owned evidence. Faults must target
explicitly owned isolated processes, never another user's active work.

Still outside this qualification: validation, constraint reduction, large
allocations, postprocessing, preconditioner application, arbitrary assembly
paths, a fixed cancellation latency inside a large row/vector operation, every
nonlinear outer loop, analytical shortcuts, external libraries/workers and child
threads. TaskIR's analytical bar path has entry/exit control, not new internal
preparation safe points. Network blackholes, multi-host partitions, power loss,
database corruption, external-solver cross-validation, material calibration and
installed Windows/macOS live cancellation remain separate work.

The 64-step batch is a polling granularity, not a latency/performance guarantee.
This round does not benchmark overhead or elevate global solver/release maturity.
Uncooperative work still owns its capacity until it actually returns.
