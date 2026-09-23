# Frame P-Delta path reliability, 2026-09-23

This is bounded local P-Delta path validation, not general nonlinear structural or collapse qualification.

Base revision: `b7ca95a2` (`daji 3.3.5`) plus the working tree, including the
preceding buckling-assembly corrections. Previous edits remain in place;
this round does not bump the version, commit, deploy or reinstall the app.

## Reproduced failures

Four public-solver regressions failed before modification:

- Rescaling an explicit imperfection direction by `1e-200` incorrectly returned
  `selected mode has no translational imperfection`, despite a finite nonzero
  shape and unchanged requested physical amplitude.
- At modal imperfection amplitude `1e-200`, the first precritical step reported
  amplification `1` instead of the secant reference `1.25`. Direct squared
  projection underflowed.
- At amplitude `1e155`, a successful linearized path contained infinite
  `max_incremental_displacement`, despite finite displacement components.
- `imperfection_mode_index = usize::MAX` panicked on unchecked index addition
  in a debug public solve instead of returning a domain error.

A private nonlinear-residual regression also failed: residual `1e308`,
reference force `1e308` and load factor `2` produced zero rather than `0.5`
because the normalization denominator overflowed.

The extreme amplitudes and stiffness scales are floating-point range fixtures,
not physically admissible large-deformation research examples.

## Corrections

Imperfection normalization first scales translation components before taking
their two-component norm. It removes the absolute direction cutoff and checks
the recovered DOFs. Equivalent product/division orders preserve the tested
finite rotational ratios without introducing a generic arithmetic framework.
Unrepresentable recovered DOFs fail explicitly.

Shared `frame_2d_stability_metrics` replaces duplicate linearized/corotational
projection and translation-norm code and also serves arc-length results.
Projection uses a scaled initial shape and excludes rotations, as before.
Finite checks cover the vectors, translation magnitudes and amplification;
they cannot publish `NaN` or infinity as successful numeric fields.

Before accepting a linearized step, each reduced row satisfies
`abs(Ku-f) / (abs(f) + sum(abs(K_ij*u_j))) <= 1e-8`, with row-local scaling.
No global force maximum hides an unrelated weak equation. The reported
residual keeps the existing `||Ku-f|| / max(||f||, 1)` meaning, evaluated
without squaring unscaled loads. This is a backward-error guard, not a
forward-error or conditioning certificate. Unrepresentable individual
recovery products are rejected rather than certified through cancellation.

The expanded branched-arch regression exposed a near-zero row with componentwise
error `1.779871e-3` after a single banded solve. Reusing the same factor for up
to three residual corrections restores the existing fixture without loosening
the `1e-8` acceptance threshold. Unresolved validation still fails. The checked
residual is reused for reporting rather than recomputed. These corrections are
internal linear refinement, not additional nonlinear equilibrium iterations.

Nonlinear residual normalization explicitly rejects nonfinite inputs and
divides by the two existing scale factors separately. The original absolute
scale floor, Newton tolerances and adaptive/nonconverged-result policy remain;
this is not an all-scale nonlinear convergence proof.

Linearized errors include the load-step index. `StabilityStep` and
`StabilityRecovery` extend existing cooperative solver observation stages;
old numeric stage codes stay unchanged. Cancellation after a completed step
is an error, not partial success. Fresh replay uses a new control token.
Initial preparation and some metric passes remain synchronous, so no strict
cancellation-latency bound or durable restart capability is claimed.

The engine contains no new physics and dispatch remains unchanged. Rust
headless plans use the existing action and task/result protocols. Existing
nonconvergence results remain diagnostic results rather than being relabeled
as successful paths.

## Coverage

- Nine public-solver tests cover shape scales `1e-200`/`1e200`, very small and
  large amplitude recovery, invalid modes/shapes, owned/borrowed agreement,
  common stiffness/load scaling `1e-160`/`1e160`, indexed step failure and
  cancellation/replay.
- Six private tests cover extreme normalization ratios, projection unaffected
  by orthogonal motion or rotations, rejected nonfinite diagnostics, corrupted
  weak equations beside strong ones, residual norms and nonlinear scale guards.
- Four CLI integration tests build actual Rust headless plans and use the real
  engine route for scale invariance, finite JSON metrics, domain errors and
  cancellation/replay. They are in-process, not remote Agent transport tests.
- Retained analytical column, portal, corotational, arc-length, material and
  branch-continuation tests protect adjacent paths. They do not expand physical
  qualification to arbitrary nonlinear branches or material histories.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test frame_2d_p_delta_reliability -p kyuubiki-cli --test frame_2d_p_delta_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib frame_2d_stability_metrics::tests
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test frame_2d_p_delta_reliability -p kyuubiki-cli --test frame_2d_p_delta_operator
./scripts/kyuubiki check-operator-validation --execute --profile frame-p-delta-local-path-reliability --out tmp/frame-p-delta-path-20260923.json
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test buckling_assembly_reliability --test frame_2d_p_delta_reliability --test frame_2d_p_delta_closed_form --test frame_2d_p_delta_explicit_imperfection --test frame_2d_material_p_delta_closed_form --test frame_2d_branch_continuation --test frame_2d_branch_controls --test frame_2d_branch_reference --test frame_2d_euler_branch_reference --test frame_2d_fiber_section_closed_form -p kyuubiki-engine --test buckling_frame_workflow --test frame_2d_p_delta_workflow
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test frame_2d_p_delta_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification results

Local macOS ARM64 results on the final working tree:

- Core protocol, solver, engine and headless-SDK library tests: 1144 passed,
  7 existing ignored, no failures.
- All 19 new tests passed in debug and release builds: 6 metric-kernel tests,
  9 public-solver tests and 4 Rust headless engine-route tests.
- Expanded mechanical regression: 71 passed across 12 targets, comprising
  the 9 new public-solver tests and 62 retained buckling, material, fiber-section,
  branch, analytical path and engine-route tests.
- The `frame-p-delta-local-path-reliability` profile executed all 5 commands
  successfully, totaling 48 test executions including retained controls.
- Strict solver all-target and CLI integration-test Clippy checks, touched-file
  formatting and whitespace checks passed.
- Profile registry validation passed with 39 profiles. This does not mean all
  39 profiles were executed in this round.
- Tensor structure/command checks, documentation inventory and project
  organization audit passed. Source/document limits remain 800/2000 with zero
  tracked line-limit debt. The shared metric module is 262 lines and the
  P-Delta entry/validation module is 597 lines.

The tensor claim is local `verified` evidence for numerical validation and
recovery only. Global status remains blocked with 4 maturity gaps,
16 evidence-grade gaps and 11 P0 gaps. No family promotion, remote benchmark,
installed-app test, version bump or Git commit was performed.
