# Mechanical and other physics output reliability, 2026-09-23

This is bounded local output-range validation, not general mechanical, electromagnetic or CFD qualification.

Base revision: `7c82708c` (`daji 3.3.4`) plus the working tree, including the
preceding thermal-contact and heat-output reliability changes. No version bump,
installed application update or remote performance campaign is implied.

## Reproduced failures

Five analytical solver regression groups initially failed:

- Two-dimensional mechanical triangle/quad patches lost nonzero von Mises
  stress at small elastic moduli; direct squares also overflowed at large
  moduli despite representable stress components.
- Three-dimensional constant-strain tetrahedra had the same equivalent-stress
  range loss. A subsequent complete-output check also found that the applied
  force scale in equilibrium diagnostics became zero at small loads and could
  overflow for large loads.
- Electrostatic electric-flux and magnetostatic magnetic-field magnitudes
  underflowed to zero or overflowed while their components were representable.
- Uniform Stokes screening velocities underflowed in the reported magnitude
  and therefore in the Reynolds-number diagnostic.
- Deliberately unrepresentable mechanical energy could return success with
  nonfinite fields serialized as `null` rather than fail the operator.

These are numerical range regressions, not claims that extreme coefficients
correspond to realistic engineering materials.

## Changes and boundaries

Mechanical plane stresses now use a scaled von Mises calculation and a stable
principal-stress recovery. The eigenvalue correction avoids cancellation of
the smaller principal stress. Three-dimensional von Mises recovery scales
deviatoric components rather than hydrostatic pressure, preserving small shear
on a large hydrostatic state. Overflowing normal-stress differences have a
separate scaled path. Displacement and force-vector magnitudes use `hypot`.

Two retained plane-patch assertions initially failed because the old
center-plus/minus-radius reference rounded a near-zero principal stress to
zero. The difference was approximately `7.28e-12 Pa` on a `1e5 Pa` tensor.
Those eigenvalue comparisons now use a tensor-norm-scaled `32 * EPSILON`
tolerance and independently check trace and determinant. Other reference
values and tolerances are unchanged; dedicated small-eigenvalue unit tests
remain strict. No physical result is clipped to zero to satisfy the oracle.

Plane and solid-tetra public solvers reject nonfinite recovered states and
energy totals. Plane energy-volume products are reordered to avoid avoidable
intermediate overflow/underflow. Nonzero energy-density contributions that
underflow during volume multiplication are rejected. Fallible result
collection and total accumulation honor the existing cancellation mechanism,
and discard partial results on error or interruption.

Electrostatic, magnetostatic and Stokes changes in this round are limited to
vector magnitudes. They do not establish a universal finite-output policy for
all of those operators' gradients, energy products or summaries. The Stokes
tests retain the existing low-Reynolds screening scope, not general CFD.

All physical checks remain in the solver/operator layer. The existing Rust
headless execution plan and engine bridge route are exercised without changes
to the engine dispatch, task protocol, payload schema or public SDK interface.

Remaining numerical boundaries include stress/strain dot-product cancellation,
energy-density underflow before volume multiplication, quadrature accumulation
range and roundoff, and conditioning of the assembled system. Mechanical
finite-strain, plasticity, contact, dynamics, modal and other operator families
are not newly qualified by these tests. Existing cohesive and thermal paths
share some stress-metric helpers but do not acquire the new public plane/solid
output checks automatically.

## New coverage

- Seven solver integration tests cover nine existing entry points: plane
  triangle/quad, solid tetra, electrostatic triangle/quad, magnetostatic
  triangle/quad and Stokes triangle/quad.
- Mechanical patches retain free degrees of freedom, use fixed strain
  `1e-3`, and vary elastic modulus through `1e-200`, `1`, `1e200`. Displacement,
  stress and strain energy are compared with closed-form values. Solid
  equilibrium load scales are checked separately.
- Electromagnetic patches prescribe a linear scalar potential `2x-y` and
  compare constitutive field magnitudes and integrated energy. Stokes patches
  prescribe uniform velocity, adapt density to keep Reynolds numbers small,
  and check zero shear, dissipation and divergence.
- Mechanical failure/replay and entry/terminal cancellation checks exercise
  node, element and total result stages for all three mechanical operators.
  The huge-strain energy-overflow fixtures intentionally lie outside the
  physical model's domain and test rejection only.
- Six new unit tests cover shear/biaxial stress, principal eigenvalues,
  hydrostatic invariance, opposite extreme normal stresses, ordered energy
  multiplication and explicit element/aggregate range errors.
- Two CLI integration tests build actual headless plans and invoke existing
  engine routes for both plane element types. They verify analytical values,
  error propagation and clean replay in process, not remote Agent transport.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test multiphysics_output_range -p kyuubiki-cli --test mechanical_output_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test multiphysics_output_range -p kyuubiki-cli --test mechanical_output_operator
./scripts/kyuubiki check-operator-validation --execute --profile plane-2d-patch-closed-form --out tmp/mechanical-plane-range-20260923.json
./scripts/kyuubiki check-operator-validation --execute --profile solid-tetra-3d-closed-form --out tmp/mechanical-solid-range-20260923.json
./scripts/kyuubiki check-operator-validation --execute --profile electromagnetic-plane-patch --out tmp/electromagnetic-range-20260923.json
./scripts/kyuubiki check-operator-validation --execute --profile stokes-flow-screening --out tmp/stokes-range-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test mechanical_output_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification results

Local macOS ARM64 results:

- Core protocol, solver, engine and Rust headless SDK units: 1130 passed,
  seven existing ignored tests.
- New solver/headless-route integration cases: nine passed in debug and nine
  in release.
- Plane-mechanical profile: all nine commands passed, 22 test executions.
- Solid-tetra profile: all nine commands passed, 23 test executions, including
  regular/warped bending convergence and component restraint/topology checks.
- Electromagnetic-plane profile: all 12 commands passed, 44 test executions,
  including orientation, refinement, input reliability, scalar-reference
  invariance and energy-to-heat projection.
- Stokes screening profile: all six commands passed, 17 test executions.
- Fifteen additional integration targets passed 106 tests: thermal-plane
  triangle/quad reviews, isoparametric/refinement/input checks, 2D/3D cohesive
  mesh closed-form/sparse/convergence checks, mechanical convergence and the
  preceding heat-contact/output regression targets.
- Strict solver all-target and mechanical CLI test-target Clippy passed with
  warnings denied. Touched Rust files pass formatting checks.
- All 35 validation profiles pass registry checks. Tensor structure and command
  checks, project organization and documentation inventory pass. Source/doc
  limits remain 800/2000 lines with zero tracked line-limit debt.
- Overall tensor readiness remains blocked: four maturity gaps, 16
  evidence-grade gaps and 11 P0 gaps. This local claim does not remove those
  broader qualification obligations.

Profile executions overlap the unit/integration runs; counts above must not
be added as unique coverage. Registration alone is not execution evidence;
the four named profiles above were actually executed in this working tree.

No installed-app, remote transport, cross-platform or large-mesh performance
claim is made. The report records executable regression evidence, not a new
physics model or an entire operator-family maturity upgrade.
