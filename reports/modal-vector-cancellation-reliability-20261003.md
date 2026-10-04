# Modal Vector Cancellation Reliability

## Scope And Reproduction

On 2026-10-03, the daji 3.4.4 source overlay based on `155d6bc6` was checked
on macOS ARM64. Sparse matrix products already observed cooperative
cancellation, but dense modal refinement still performed complete vector
scans, reductions and updates without checking inside those operations.
Four initial regression tests failed before that gap was fixed. This report
records bounded cancellation and recovery verification, not an expansion of
the supported modal spectrum or an installed desktop validation.

The 128-element planar bending request for six modes still fails its original
refinement gate. Local smoothing experiments did not meet that gate and were
removed. The existing public, headless and live Agent unresolved-spectrum
regressions still require explicit failure without partial modes and a valid
subsequent request. No success is claimed for the unresolved request.

## Implementation Boundary

`modal_frame_refinement.rs` now checks vector validation, norm accumulation,
compensated dot products, residual construction, binary scaling, projection,
inverse correction and smoothing updates at entry, every 64 components and
the final partial block. The new `modal_vector_scan`, `modal_vector_dot` and
`modal_vector_update` stages are appended to `SolverStage`; existing numeric
stage identifiers are not renumbered. Cancellation returns from the numerical
operation itself, not only from its enclosing control scope.

`linear_sparse_residual.rs` supplies a cancellable norm that retains the same
scaled accumulator and term order as the existing stable norm. No partial
norms are combined or used after interruption. Compensated dot products also
retain their original term order and now reject mismatched dimensions instead
of truncating pairs. Invalid Rayleigh values fail explicitly.

The correction algorithm remains in Solver. Engine and Agent do not implement
another numerical algorithm. The four inverse corrections, single smoothing
candidate per correction, prepared-factor reuse, spectral-neighborhood guard,
physical output checks and numerical tolerances remain unchanged. There is no
new tolerance floor, numerical dependency, factor cache or hidden retry loop.

Agent fault injection admits the three new stages only through its existing
explicit hold-path, supported-method and exact-job-marker controls. This lets
development tests cancel a real TaskIR while it is held at a vector safe point;
it does not enable a default runtime hold or a new production service.

Private in-place refinement vectors may be partly updated on interruption.
They are not resumable checkpoints and are never published as successful
modes. Replay uses a fresh request/control token, not the interrupted vector.
Smoothing candidates remain private until their true residual improves, so
cancellation in that helper leaves the caller's seed and eigenvalue unchanged.

## Regression And Live Recovery

The refinement target passes 21 tests, including nine new vector-control
tests. They check the 64-component cancellation boundary, stop consuming a
4097-term norm after the first 64 terms, verify the final partial checkpoint,
and preserve sampled norm bits for lengths 0, 1, 63, 64, 65, 127, 128, 129 and
4097 at normal, very large, very small and subnormal scales. The cancellation
boundary does not regroup a compensated dot product. Dimension truncation,
non-finite products and invalid Rayleigh values fail explicitly.

Later-phase checks arm cancellation only after a basis dot or prepared inverse
substitution has finished. The subsequent projection or inverse update returns
cancellation after 64 components and does not finish the vector. Separate
replays pass the unchanged residual checks. Existing budget, factor-reuse,
binary scaling and repeated-direction regressions remain part of this target.

The public mass-coordinate target passes 26 tests. Two new tests cancel the
100-element planar and spatial fixtures in each of the three vector stages,
then solve the same immutable input again. Replay checks independently
assembled physical residuals, discrete roots, restrained zeros, unit shape
norms and mass orthogonality. Spatial replay retains paired bending modes.

The live local TCP Agent target passes 11 tests. Two new tests cover all three
stages for each of the planar and spatial TaskIRs. Cancellation returns an
error with a non-resumable numerical checkpoint and no result, releases the
execution slot, and leaves the Agent accepting subsequent jobs. A fresh
TaskIR on the same Agent passes independently checked physical residuals.
The hold marker remains present, verifying exact-job rather than global holds.

These focused lanes overlap with the modal profile and must not be added as
distinct coverage. They do not replace operational or industrial qualification.

## Validation Results

| Lane | Result |
| --- | --- |
| Full Solver suite including integration and doc harnesses | 1321 passed, 0 failed, 9 existing ignored; 186 result groups |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Executed modal profile | 30 commands, 292 passed, 0 failed, 0 ignored |
| Strict Solver and CLI Clippy on all targets | Passed with warnings denied |
| Formatting and whitespace | Passed |
| Documentation book and inventory | Passed |
| Source 800 and document 2000 organization audit | Passed; zero tracked debt |

The executed profile has `executed=true` and `ok=true`; its saved-report gate
also passes. It includes the three Agent fault-admission tests and the expanded
refinement, public replay and live Agent targets, alongside existing headless,
TaskIR and unresolved-spectrum regressions. No ignored test was added. This
does not claim execution of the entire CLI test suite or any installed package.

Reproduce from the repository root with the native validation entrypoint:

```sh
CARGO_NET_OFFLINE=true ./scripts/kyuubiki check-operator-validation --execute --profile modal-frame-sanity --out tmp/modal-vector-cancellation-validation.json
CARGO_NET_OFFLINE=true ./scripts/kyuubiki check-operator-validation --in tmp/modal-vector-cancellation-validation.json --profile modal-frame-sanity
```

The local JSON is ignored runtime evidence, not a new retained qualification.
The tensor links separate verified Solver and development-Agent claims for
numerical checks and recovery. Structural and command validation pass with
13 modules, 11 paradigms and no structural gaps. Daji readiness stays blocked:
four maturity gaps, sixteen evidence-grade gaps and eleven P0 gaps remain.

Historical reports retain their original source versions and fingerprints.
The tridiagonal preparation changes from the preceding round are preserved;
this report describes only the subsequent vector-control overlay.

## Source Fingerprints

Final SHA-256 values are relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_frame_refinement.rs` | `6240a6da67a6a78ff163cad9463d05fb4e0421734518ff212025be62da09dc14` |
| `solver/src/modal_frame_refinement_control_tests.rs` | `85a60600ce4e20449fafc9f52fca443d4aed1add5ec10391ae1beb99369a93ee` |
| `solver/src/linear_sparse_residual.rs` | `ea807e4fe02a7d62856e5e1d3e88e20b4186577902f9fc1c2c4c9f12b1ba93e7` |
| `solver/src/solver_control.rs` | `0605b3bb7b9ebb2b319f35e87e15b203d6a42ef423f6cbdebf789f777298b997` |
| `solver/tests/modal_mass_scaling_reliability/control.rs` | `006667b40c1ef6dd05b1e547c583ff7dae1253ffcdf7bfcd80bca82c4c939bcc` |
| `solver/tests/modal_mass_scaling_reliability/spatial.rs` | `c156c302db429f3cba279d25251c14b4d19e13e63cf07ac99a00a85d997a3a73` |
| `cli/src/agent_fault_injection.rs` | `7c219198fd8089ff6f018bbd85e794a6eba2aba58da6290635d4f0931decd9c8` |
| `cli/tests/agent_modal_live.rs` | `52988e74242a68527e32c013cbcb620e8f501c70749512c3bac0127868f3e1c5` |

## Remaining Boundaries

This is bounded modal-vector cancellation verification, not general modal or Agent qualification.
Cooperative checks bound numerical components between polls, not wall-clock
latency during allocation or arbitrary callback execution. This round adds no
million-DOF bending proof, runtime throughput claim, installed desktop proof
or remote Linux verification. Source metadata remains 3.4.4; no desktop package has
been rebuilt or reinstalled, and no commit or push is part of this round.
