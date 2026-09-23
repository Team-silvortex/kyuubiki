# Adaptive frame fiber-integration reliability, 2026-09-23

This is bounded local adaptive fiber-integration validation, not a global discretization-error certificate or general damage/collapse qualification.

Base revision: `b7ca95a2` (`daji 3.3.5`) plus the preceding working-tree
buckling, P-Delta and material-history corrections. Existing edits are
preserved. There is no version bump, commit, deployment or app rebuild.

## Reproduced Defects

Five new regressions failed before modification:

- A common force scaling of `1e-200` turned a nonzero comparison error into
  zero because the old Euclidean norm squared unscaled values. Large finite
  scales also overflowed the old norm.
- An axial force of `1e12` masked a 50% error in one end moment, reporting
  `5e-13`. Force and moment values were mixed into a single normalization.
- A plastic section changed from 12-point to 2-point integration when its
  force scale changed, without changing the dimensionless physical state.
- Changing the length unit by `1e-6` changed the same section's reported
  estimate from about `0.006207` to `0.040546`.
- When all fibers had their own material overrides, an unused parent yield
  strength of `1e30` still changed the selected rule from 12 points to 2.

The extreme scales are floating-point range controls. They are not experimental
material data or a claim of unrestricted full-solver unit/scale invariance.

## Correction

Each fiber section accumulates the absolute integration contributions for
axial force and for each end moment. These three private values are checked
for finiteness; they are not new public protocol fields.

For each component, let `S` be the maximum of both result magnitudes and both
absolute contribution sums. A zero `S` gives zero discrepancy. Otherwise
compute `d = abs(candidate/S - reference/S)`. Differences no greater than
`8 * machine_epsilon * (max_candidate_fiber_points + 1)` are treated as local
accumulation roundoff. For remaining differences, the estimate is
`d / max(abs(candidate/S), abs(reference/S), 1e-12)`. The reported candidate
comparison is the maximum of the three component estimates.

This is a component-local floating-point resolution allowance, not a formal
error bound for the constitutive law or a relaxation of the requested
integration tolerance. It avoids unscaled subtraction/squares and prevents a
different force or moment component from determining the denominator. Parent
material defaults do not participate in the estimate; the actual integrated
fiber responses determine the scales.

Every 2/3/4/8/12-point candidate is validated before comparison or selection.
A failed candidate returns a domain error containing its order, even when a
lower-order result appears usable. An injected finite high-order history
that makes only the 12-point force overflow is rejected, rather than allowing
`NaN` comparisons to be masked by a lower-order result. Fresh replay is also
checked. This injection is an internal negative control, not an exposed
material-checkpoint import feature.

The serialized `longitudinal_integration_error` key and engine dispatch/task
formats are unchanged. The metric's numerical meaning is now the maximum
componentwise estimate, so historical Euclidean values are not directly
comparable. The 12-point cap is unchanged; its estimate may exceed tolerance
and remains visible. Newton convergence does not certify integration accuracy.
Global mesh error, tangent error, softening localization, fatigue and durable
restart are outside this correction.

## Coverage

- Eight new kernel tests cover force scales `1e-200`/`1e200`, weak bending
  beside strong axial force, unused parent defaults, length units
  `1e-6`/`1e6`, nonfinite comparisons, local roundoff, overflow rejection with
  candidate order and replay. Opposite `f64::MAX` comparison remains finite.
- Two new public-solver tests cover axial-bending plastic paths under length
  units `1e-3`/`1e3` and common stiffness/load/yield scaling `1e145`/`1e160`.
- Two new Rust headless-plan tests use the real engine route for cyclic
  mixed-fiber diagnostics at a `1e160` scale and exact output independence
  from fully overridden parent material defaults.
- Retained independent dense cyclic, fiber damage, quadrature, section-library,
  nonlinear material, P-Delta and branch-workflow tests protect adjacent paths.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib frame_2d_fiber
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test frame_2d_fiber_section_closed_form -p kyuubiki-cli --test frame_2d_material_operator
./scripts/kyuubiki check-operator-validation --execute --profile frame-fiber-integration-local-reliability --out tmp/frame-fiber-adaptive-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test frame_2d_material_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification

Local macOS ARM64 results on this working tree:

- Core protocol, solver, engine and headless-SDK libraries: 1161 passed,
  7 existing ignored, no failures.
- All 12 new tests passed in debug and release: 8 kernel, 2 public-solver
  and 2 in-process Rust headless route tests. The release groups also passed
  23 retained fiber, damage, quadrature, cyclic and material-route controls.
- Expanded mechanical regression: 67 passed across 11 public-solver/engine
  targets. Together with the 5 headless material tests, 72 tests passed across
  12 targets, including the 4 new public/headless cases.
- The `frame-fiber-integration-local-reliability` profile executed all 4
  commands successfully, totaling 53 test executions including retained
  reference cases. This is not execution of the entire registry.
- Strict solver all-target and CLI integration-test Clippy checks, touched
  source formatting and whitespace checks passed.
- Registry validation passed with 41 profiles. Tensor structure/command
  checks, documentation inventory and project organization audit passed.
  Source/document limits remain 800/2000 with zero tracked line-limit debt.

The global tensor still reports 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps; the Daji qualification status remains blocked. No broader release
gate is closed by this local evidence.

Only local numerical-validation and recovery evidence is registered. No
remote Agent transport, GUI test, performance gain, broad physics qualification
or global release-gate closure is claimed.
