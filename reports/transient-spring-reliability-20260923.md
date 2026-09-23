# Transient spring reliability, 2026-09-23

This is bounded local transient-spring validation, not general dynamic FEM qualification.

Base revision: `7c82708c` (`daji 3.3.4`) plus the working tree, including the
preceding thermal, multiphysics output and modal reliability changes. No
version bump, installed-app update or remote benchmark is implied.

## Reproduced failures

Three new regression tests failed before the fix, while two independent
energy/work controls already passed:

- A statically preloaded oscillator (`m=2`, `k=100`, `c=0.5`, `f=10`, `u=0.1`,
  `v=0`) acquired velocity about `5.55e-7` after 20 steps of size `1e-10`.
  The true state is constant. Recovering acceleration by subtracting two
  nearly equal total displacements amplified roundoff by `1/dt^2`.
- Two unconstrained masses translating together from displacement `1000`
  at velocity `1`, with time step `1e-7`, returned velocity about
  `1.00000454747` rather than preserving rigid motion. The spring and damper
  should see no relative movement.
- An undamped oscillator with deliberately extreme force `1.2e154` had an
  unrepresentable strain energy near half a cycle, but finite endpoint energy
  near a full cycle. Full history rejected it; `history_stride=63` returned
  success because intermediate energy validation was skipped. This fixture
  tests numerical rejection, not physically meaningful material data.

## Solver changes

The constant-load, linear average-acceleration Newmark equations are now solved
in an algebraically equivalent velocity-increment form, with `h=dt`:

```text
(K + 2 C/h + 4 M/h^2) delta_v = (4/h) M a_n - 2 K v_n
v_(n+1) = v_n + delta_v
u_(n+1) = u_n + h (v_n + delta_v/2)
a_(n+1) = M^-1 (f - K u_(n+1) - C v_(n+1))
```

The effective sparse factorization is still built once and reused. Acceleration
comes from equilibrium, not a subtraction of nearly equal total displacements.
The existing finite-coefficient, inertia and right-hand-side range checks
remain. This does not add time-dependent forcing or adaptive time steps.

Every time step now computes finite energy and peak displacement/velocity
statistics. Only retained steps clone the state into history. Thus sampling
reduces output storage without changing validation or the calculated trajectory.
Errors include the failing time-step index for state and energy recovery.
Unrepresentable total simulation duration is rejected before numerical work,
rather than after advancing a potentially enormous number of unsaved steps.
Combined final spring/damper forces are checked for overflow before summary
construction.

Step, state and energy checkpoints use the existing cooperative cancellation
mechanism; final node and element recovery use the shared cancellable result
collectors. Three solver progress-stage variants are appended without changing
existing numeric codes. The engine dispatcher, task format and SDK payloads
are unchanged; all physical logic remains in the solver/operator layer.

## Coverage and boundaries

- Ten solver integration tests cover the three reproduced failures, invalid
  total-duration preflight, 2000-step undamped energy conservation, damped
  work/dissipation balance, independent
  underdamped analytical response with second-order time refinement, a
  heterogeneous multi-mass chain, sampled-history equivalence, state-based
  continuation and cancellation/replay. A test can cover several conditions.
- The heterogeneous chain checks discrete work balance and midpoint kinematics.
  Sampling preserves final nodes, retained frames and peak statistics exactly
  in the tested run. A 137-step plus 263-step constant-load continuation matches
  the uninterrupted 400-step final state.
- Cancellation is tested inside the solver at five stages, including a step
  that is not retained in history. Interrupted calls cannot return a partial
  success, and replay matches an uninterrupted baseline.
- Three CLI integration tests build actual Rust headless plans and resolve
  existing engine routes for static invariance, intermediate failure/replay
  and feeding returned state into a subsequent batch. These are in-process
  calls, not remote Agent transport or durable checkpoint files.
- Retained tests include the prepared sparse path on a 10,000-node chain, the
  existing single-step Newmark reference and undamped time refinement. No new
  million-node performance or overhead claim is made.

This scope is a linear lumped-mass spring/damper model with fixed constraints
and constant applied loads. It does not qualify transient 2D/3D solids,
nonlinear contact, impact, changing loads or damping laws, nor prove accuracy
at all coefficient scales. Per-step checks reject nonfinite energy but do not
certify arbitrary subnormal energy products or accumulation conditioning.
The finite Newmark coefficient range remains bounded. Added validation and
equilibrium recovery have not received a new large-scale overhead benchmark.
`max_force` remains the final-state maximum, not the maximum over time.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test transient_spring_balance -p kyuubiki-cli --test transient_spring_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test transient_spring_balance -p kyuubiki-cli --test transient_spring_operator
./scripts/kyuubiki check-operator-validation --execute --profile transient-spring-local-reliability --out tmp/transient-spring-20260923.json
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test mechanical_convergence --test modal_spectrum_reliability --test transient_heat_bar_reliability --test transient_heat_bar_closed_form
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test transient_spring_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification results

Local macOS ARM64 results:

- Core protocol, solver, engine and Rust headless SDK units: 1133 passed,
  seven existing ignored tests.
- New solver and headless-route integration tests: 13 passed in debug and
  13 in release.
- The new `transient-spring-local-reliability` component profile executed
  all four commands successfully, with 21 test executions. This includes
  retained transient input/history, 10,000-node sparse-chain and dynamic
  closed-form checks; no release-candidate qualification was added.
- Four additional solver integration targets passed 38 tests: mechanical
  convergence, modal-spectrum reliability, transient heat-bar closed form
  and transient heat-bar reliability.
- Strict solver all-target and transient CLI test-target Clippy passed with
  warnings denied. Touched Rust files pass formatting checks.
- All 36 validation profiles pass registry checks. Tensor structure and
  command checks, project organization and documentation inventory pass.
  Source/doc limits remain 800/2000 lines with zero tracked line-limit debt.
- Overall tensor readiness remains blocked: four maturity gaps, 16
  evidence-grade gaps and 11 P0 gaps. The new evidence is a scoped `verified`
  claim, not a blanket mechanical-family maturity upgrade.

Profile executions overlap the unit/integration runs and must not be summed
as unique coverage. Prior reports retain the counts from their own runs.
No installed-app, remote transport or large-scale timing run was performed.
