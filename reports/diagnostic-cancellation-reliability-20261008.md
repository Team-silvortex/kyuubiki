# Diagnostic Cancellation Reliability

Date: 2026-10-08. Source macOS ARM64 verification of shared result-record
reductions and native Headless -> Orchestra -> Rust Agent cancellation. This
extends the [stabilized transport research chain](advection-diffusion-research-chain-20261008.md)
without changing its physics, scoring defaults or historical qualification.

## Reproduced Gap and Repair

Three new tests failed before the repair: node scans continued past a requested
cancellation, the stabilization scan could not stop after earlier physical peaks,
and empty/short/chunked/final scans had no observable safe points. A fourth
original-error preservation regression already passed and remains enforced.

`Samples::scan` now uses the existing execution-control scope at entry, every
64 successful records and the final record. `ResultDiagnostics` is appended
to the existing stage enum, preserving prior stage values. The control helper
keeps scan counts local; cancellation discards partial summaries rather than
exporting a resumable checkpoint. No second watchdog or GUI dependency is added.

The native RPC wrapper uses `execute_diagnostics` for an observed diagnostic
interruption and retains `execute_solver` for existing numerical interruptions.
Terminal-cause ownership, task identity, exact-target cancellation and final
publication checks remain in their existing runtime paths.

## Executed Native Chain

The live test starts an isolated Orchestra and a capacity-one Rust Agent. Its
diagnostic input is an actual 256-element upwind solver output, with 257 nodes,
unit area/diffusivity and velocity 20. An explicitly enabled, task-scoped hold
pauses `result_diagnostics` after 64 records. The public Rust SDK inspects the
owned dispatch and requests cancellation with its exact process, request,
generation and job identity.

The Agent returns a task-bound cancelled failure, no partial diagnostic result
and no permission to retry. The following project-create action never executes.
The original dispatch remains `observed_failed`, with no completed result. One
explicit rerun of the identical TaskIR then completes on the same Agent, with
257 nodes, 256 elements and artificial diffusivity `20/(2*256) = 0.0390625`.
Both attempts retain distinct identities; the old failure is not relabelled as
the new success. Exactly two Agent executions occur, with no hidden replay.

The engine regressions also interrupt the fifth element pass, after physical
peaks were already accumulated, and require no summary to escape. Fresh healthy
execution recovers. Cadence checks cover lengths 0, 1, 63, 64, 65 and 129, without
duplicate final safe points. A sample error encountered before cancellation is
observed retains its original cause instead of becoming `cancelled`.

## Verification

All runs below used the current source on macOS ARM64, with offline Rust
dependencies and isolated loopback services for the native live tests:

| Suite | Result |
| --- | --- |
| Engine library, including four new cancellation regressions | 649 passed, 1 existing ignored |
| Thermo/transport integrity and stabilization workflow integration | 7 + 2 passed |
| Solver execution-control regressions | 12 passed |
| CLI/Agent unit regressions | 186 passed |
| Exact execution and solver cancellation live suites | 4 + 52 passed |
| Complete native Headless live suite, including diagnostic cancellation | 61 passed |
| Transport validation profile | 9 commands, 51 tests passed |

The ignored engine test is `loads_prebuilt_template_cdylib_through_dynamic_host`;
it requires a separately built operator-template dynamic library. This round
does not claim to exercise that plugin. Suite counts include shared fixture
checks and overlap with profile tests; they are not summed into unique coverage.

Strict all-target Clippy passed with warnings denied for Protocol, Solver,
Engine, CLI and Headless SDK. Formatting and diff whitespace checks passed.
Native topology, module-function matrix, extension standard, runtime API
surface, tensor/self-test, documentation inventory/book, project organization,
version contracts and validation-profile structure checks passed. Source and
documentation limits remain 800 and 2000 lines, with no tracked size debt.

The transport profile includes the four new engine regressions. Its generated
validation receipt is kept under ignored `tmp/` and checked again through the
native report validator; no large test output or deployment configuration is
added to Git. The tensor has zero structural gaps but still reports four
maturity, 19 evidence-grade and 14 P0 gaps, with Daji acceptance blocked. This
bounded cancellation evidence does not promote historical qualification.

## Boundaries

Safe points cover shared borrowed-record `Samples::scan` reductions. They do not
make all extractors, JSON decode/clone/digest, output serialization, dynamic
plugins or child threads preemptible. Polling every 64 records bounds record
cadence, not wall-clock latency. This small deterministic service test is not a
1M-model benchmark, remote/installed qualification or authenticated provenance.
The exact cancellation fixture uses a standalone TaskIR batch; it does not by
itself qualify every workflow-graph job cancellation path. Durable restart and
numerical continuation remain outside this evidence.

Usage and receipt semantics:
[cooperative diagnosis cancellation](../docs/advection-diffusion-research-chain.md#cooperative-diagnosis-cancellation).
