# Modal spectrum reliability, 2026-09-23

This is bounded local modal-spectrum validation, not general mechanical or large-mesh qualification.

Base revision: `7c82708c` (`daji 3.3.4`) plus the working tree, including the
preceding thermal and multiphysics output-reliability changes. No version bump,
installed application update or remote performance campaign is implied.

## Reproduced failures

Four initial solver regression tests failed before the changes:

- Two independently anchored axial frame branches with unit area, density and
  length and elastic moduli `1` and `1e14` should have eigenvalues `2` and
  `2e14`. Asking for two modes discarded the soft eigenvalue because the
  positive-mode filter used a fraction of the largest eigenvalue. Asking for
  one mode retained it. Both 2D and 3D operators were affected.
- With bending released on those 2D cantilevers and second moment `0.01`, the
  soft bending eigenvalues are `0.01 * (60 +/- 12 * sqrt(21))`, followed by the
  axial eigenvalue `2`. The original result started near `5.009e12`, not
  `0.05009`. Besides filtering, the global Jacobi coupling threshold suppressed
  the soft block's off-diagonal terms. Removing the filter alone is insufficient.
- A unit 3D frame along `(1,1,1)/sqrt(3)`, with its root clamped and tip
  rotations fixed, has translational eigenvalues `240`, `240`, `2000` for
  `E=1000`, `A=rho=1`, `Iy=Iz=0.01`. A single-mode request returned the uniform
  axial eigenvector at `2000`; the full spectrum correctly started at `240`.
  A zero eigenpair residual did not establish that it was the lowest mode.
- A floating component beside an anchored component passed the global support
  count check. Its zero mode was filtered out and the remaining positive
  spectrum was reported as success instead of a restraint error.

These references describe the existing lumped-mass discrete frame equations,
not a new continuum model or experimental material validation.

## Changes

Both frame operators now share spectrum selection and result checks. The
restrained-frame contract checks rigid-motion restraint rank independently for
each connected component and rejects nodes with no element mass. Distributed
supports are accepted without requiring a fully clamped node. Nonpositive or
nonfinite eigenvalues are errors, not entries silently removed from the spectrum.

The symmetric Jacobi solver uses a diagonal-pair-relative coupling threshold
and a stable rotation parameter. This preserves resolved soft disconnected
blocks without letting a stiff block set their stopping threshold. Returned
frame modes receive an independent sparse-operator residual check relative to
their own eigenpair scale. Shape, period and aggregate mass checks reject
unrepresentable outputs instead of allowing successful nonnumeric JSON fields.

For single-mode requests, recognized tridiagonal/axial models retain their
linear-memory path. Other problems with at most 128 free DOFs use a complete
sorted spectrum. Larger general sparse problems use uniform and deterministic
nonuniform probes; both must converge and the lower eigenvalue is selected.
Failure of the second probe cannot be concealed by convergence of the first.
Additional checkpoints interrupt Jacobi sweeps, sparse iterations and spectrum
validation; tests verify errors inside the solver, before observer-scope exit.

Operator-specific restraint and spectrum logic stays in the solver crate. The
engine dispatch, task representation and headless payload schemas are unchanged.
Three solver progress-stage enum variants are appended, preserving existing
stage codes. SDK-to-engine integration tests use the existing manifest route.

## Coverage and limitations

- Nine new solver integration tests cover soft/stiff branches, analytical
  bending, rotated frames, per-component constraints, distributed supports,
  orphans, node/member permutation, aggregate mass overflow and cancellation
  followed by successful replay. Several tests cover both 2D and 3D.
- The rotated-frame test includes 44 independent beams and 132 free DOFs, so
  its single-mode solve really exercises the general sparse route rather than
  only the new bounded dense check. It compares against both analytical
  eigenvalues and the complete-spectrum route.
- Three new sparse-iteration unit tests cover escape from a uniform higher
  mode, refusal to hide an unconverged second probe and cancellation/replay.
- Two new CLI integration tests build Rust headless execution plans, resolve
  the actual bridge route, invoke the engine and check numerical results or
  operator error propagation followed by correction/replay. This is
  in-process integration, not remote Agent transport.
- The shared Jacobi changes require retained buckling and mechanical
  convergence regression; those families are not newly qualified here.

Two deterministic probes and a small residual are not a mathematical
lowest-eigenvalue certificate for arbitrary large sparse systems. Nearly
clustered spectra can still exhaust inverse iteration; the operator reports
failure instead of returning an unconverged result. Large general multi-mode
problems still encounter the existing dense limit of 4096 reduced DOFs.
Free-free analysis and rigid-mode reporting are not introduced. The existing
recognized long axial-chain residual floor (`2e-4`) remains separately scoped;
general inverse iteration retains `1e-6`, and dense frame results use `1e-8`.
No new million-node timing, material certification, installed-app or
cross-platform claim is made. Added topology checks and the second probe have
not received a new large-model overhead benchmark in this round.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test modal_spectrum_reliability -p kyuubiki-cli --test modal_spectrum_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test modal_spectrum_reliability -p kyuubiki-cli --test modal_spectrum_operator
./scripts/kyuubiki check-operator-validation --execute --profile modal-frame-sanity --out tmp/modal-spectrum-20260923.json
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test operator_modal_reliability --test mechanical_convergence --test buckling_beam_1d_closed_form --test buckling_frame_2d_closed_form --test buckling_frame_2d_portal --test buckling_frame_2d_clustered_modes --test buckling_beam_1d_clustered_mode --test buckling_beam_frame_crosscheck --test buckling_beam_1d_input_reliability --test buckling_frame_2d_input_reliability
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test modal_spectrum_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification results

Local macOS ARM64 results:

- Core protocol, solver, engine and Rust headless SDK units: 1133 passed,
  seven existing ignored tests.
- New solver and headless-route integration cases: 11 passed in debug and
  11 in release, including the 132-free-DOF sparse case.
- The registered `modal-frame-sanity` profile executed all seven commands
  successfully, with 26 test executions including retained modal checks.
- Ten additional solver integration targets passed 43 tests: retained modal
  operator reliability, mechanical convergence and eight beam/frame buckling
  targets (closed-form, clustered modes, cross-check, portal and invalid input).
- Strict solver all-target and modal CLI test-target Clippy passed with
  warnings denied. Touched Rust files pass formatting checks.
- All 35 validation profiles pass registry checks. Tensor structure and
  command checks, project organization and documentation inventory pass.
  Source/doc limits remain 800/2000 lines with zero tracked line-limit debt.
- Overall tensor readiness remains blocked: four maturity gaps, 16
  evidence-grade gaps and 11 P0 gaps. This is a scoped `verified` claim, not
  an upgrade of the entire mechanical family or a resolution of those gaps.

Profile executions overlap the unit/integration runs; counts above must not
be added as unique coverage. Prior reports remain historical records of their
own test runs, not counts for this change. No remote or installed-app run was
performed.
