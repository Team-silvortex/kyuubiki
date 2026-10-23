# Original Newton-Increment Equation Admission

Date: 2026-10-23

Version line: daji 3.6.x, after `0a65f2e8`. This extends the uncommitted
[Newmark gate](newmark-original-equation-admission-20261023.md) to the shared
nonlinear spring and gap-contact load path without treating nonlinear states
as static tangent solutions.

## Reproduced Gaps

Three internal-state tests failed before production edits. The old evaluator
accepted a zero-free-residual state despite a translated fixed support,
incomplete/extra displacement slots, or non-finite displacement on an isolated
fixed node. Even a signed minimum-subnormal fixed drift escaped the zero-residual
branch. These are injected internal states, not demonstrations that ordinary
public requests can currently supply or create arbitrary internal displacements.

The Newton path also verified only its reduced linear solve. It had no complete
pre-elimination increment admission before the direction entered line search.
This is a missing defensive contract, not evidence that every prior Newton
result was wrong.

## Two Separate Contracts

Every `SpringStep::evaluate` now requires the complete displacement dimension,
finite values at every node and exact zero at fixed components before assembly
or early convergence. Signed zeros remain equal. Loaded restraints are permitted;
their physical reactions do not have to vanish. The existing nonlinear residual
and caller's absolute force tolerance still determine physical convergence.

Every computed full, unscaled Newton direction must additionally satisfy:

```text
K_t(u_n) delta_u = load_factor * f_applied - f_internal(u_n)
delta_u[fixed] = 0
```

The increment RHS is the already assembled current residual, with fixed slots
excluded from equilibrium. The shared `PrescribedSystemValidator` checks the
pre-elimination tangent, complete increment and full current residual, including
the existing finite-input, matrix-structure and range policies. Support equations
may carry incremental reactions. There is no SPD requirement in admission: an
invertible indefinite tangent that the existing pivoted solver handles remains
admissible.

Do not test `K_t(u_n) u_n = f_applied`; that is not the nonlinear equilibrium
equation. Likewise, line search may scale the validated direction. The gate
checks the full Newton direction before damping, not the scaled candidate
against the unchanged Newton RHS. Each candidate then follows the original
nonlinear assembly/residual, merit-decrease and convergence checks.

## Recovery and Cost

The complete increment occupies the existing candidate scratch buffer. It is
cleared with cancellable batches, filled through bounds-checked mapping and
validated before the line search overwrites it from the base displacement.
Incorrect dimensions, out-of-range indices, fixed drift, non-finite operands or
equation failure return an error; there is no new tolerance relaxation or fallback.

Each increment prepares a temporary O(n) constraint index and scans O(n+nnz)
original data. Each nonlinear evaluation adds an O(n) state scan. No extra full
increment vector, matrix copy or factorization is retained. The reduced matrix
and RHS are still dropped before admission; the old tangent and its temporary
index are dropped before candidate merit assembly, preserving the existing
buffer/old-state lifetime strategy. No speed or peak-RSS improvement is claimed.

Numerical admission errors and cooperative cancellation propagate as errors, not
partial convergence. Ordinary iteration/backtracking budget exhaustion still
returns the last committed load/state with failed-trial diagnostics. The engine,
task IR, operator input/output formats and persistence semantics are unchanged.

## New Tests

Fourteen tests comprise seven solver unit cases and seven engine integrations:

- The three zero-residual state controls above, including exact fixed identity,
  complete dimensions and isolated hidden values, followed by clean reuse.
- Current nonlinear tangents and active-contact penalties at binary coefficient
  scales `2^-600`, `1`, `2^600`; a manufactured increment, wrong reconstruction,
  fixed-component scatter and altered original residual controls.
- A touching-contact softening state with invertible reduced tangent
  `[[0, -1], [-1, 1]]`, requiring pivoting rather than an SPD-only gate.
- Invalid complete/delta dimensions, free indices, non-finite unknowns, hidden
  constrained RHS data and invalid constrained matrix entries.
- Cancellation during scratch clearing, support indexing, full-input checks,
  a 2,050-entry original row and final admission, then scratch reuse.
- Serialized direct/workflow equality and one gate entry/completion per computed
  Newton correction, rather than one per line-search trial or stored load step.
- Cancellation in load step three after earlier steps committed, through both
  operator families and entry/input/completion stages, followed by equal replay.
- Loaded all-fixed, zero-correction models retain state checks; failed load steps
  retain their prior committed physical state and later larger budgets recover.
- Manufactured branched hardening and active-contact models at signed displacement
  components and binary force scales `2^-200`, `1`, `2^200`, reaching the general
  non-path factorization rather than only the chain shortcut.
- A 1,030-node restrained star reaches a genuine wide original row; direct and
  workflow calls cancel before line search in the second load step and replay.

The star fixtures exercise bounded cancellation, not million-node qualification.

## Validation

Pinned Rust 1.88.0, offline locked dependencies, existing build roots:

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed |
| --- | ---: | ---: |
| Solver library excluding `modal_frame_spectrum::` | 762 | 762 |
| Engine library | 711 | 711 |
| Twenty-five solver/engine integration targets | 157 | 157 |
| Non-overlapping total | 1,630 | 1,630 |

Each platform has zero final failures, ten ignored library cases and 292 filtered
modal tests. The fourteen new cases are included once; focused runs are not added
to the total. All twelve existing nonlinear solver targets, three existing
nonlinear workflow targets and the preceding nine-target constraint/transient
scope were rerun along with the new integration target. Existing globalization,
extreme force/length units, softening, contact switches, partial-load rollback,
energy balance and 10,000-node transient-chain coverage remain green.

Local solver/engine all-target Clippy passes with warnings denied; workspace
formatting and whitespace checks pass. The three source/test files were copied
to the existing Linux test tree without deleting remote files, deploying services
or changing credentials/configuration. Existing build roots were reused. No
application package, backup tree, version bump, staging operation or commit was
created for this operator work. Source/configuration files remain under 800
lines and documentation under 2,000 lines.

Native operator metadata checks pass for 85 profiles with `executed=false`. The
new profile's three registered commands also pass with `--execute`; those reruns
are not counted again. Tensor structure and command checks pass for 13 modules
and 11 paradigms, with zero structural gaps, two maturity gaps, 23 evidence-grade
gaps and 18 P0 gaps. Overall readiness remains `blocked`; this local contract does
not promote the complete nonlinear solver or engine to qualified status.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib newton_original -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib \
  --test nonlinear_constraint_admission --test nonlinear_spring_workflow \
  --test nonlinear_spring_topology_workflow --test nonlinear_spring_recovery_workflow \
  --test nonlinear_spring_equation_scaling --test nonlinear_spring_softening_arithmetic \
  --test nonlinear_spring_topology --test nonlinear_spring_range_reliability \
  --test nonlinear_spring_globalization --test nonlinear_spring_recovery \
  --test nonlinear_spring_scaling --test nonlinear_spring_tolerance \
  --test nonlinear_spring_1d_closed_form --test nonlinear_spring_tangent_pivot \
  --test nonlinear_spring_reliability --test nonlinear_spring_1d_review \
  --test transient_spring_constraint_admission --test transient_spring_reliability \
  --test transient_spring_balance --test dynamic_spring_closed_form \
  --test mechanical_constraint_admission --test thermal_frame_constraint_admission \
  --test magnetostatic_prescribed_workflow --test transient_heat_bar_reliability \
  --test advection_diffusion_bar_reliability --no-fail-fast \
  -- --skip modal_frame_spectrum:: --quiet
```

## Limits

The increment gate borrows the assembled f64 tangent and residual; it does not
independently derive constitutive laws or assemble another physical reference.
The state check does not prove nonlinear uniqueness, global convergence, stability,
forward accuracy or general contact complementarity. Other nonlinear formulations,
harmonic/complex and eigen problems require their own contracts. Shared extreme
range failures remain conservative rather than arbitrary precision. Cooperative
cancellation/replay and partial load rollback are not durable process recovery.

This is bounded original Newton-increment equation admission and nonlinear-state constraint validation, not general nonlinear qualification, forward accuracy, independent assembly or durable restart.
