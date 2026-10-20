# Spring and Gap Contact Unit-Scaling Validation

Date: 2026-10-20

## Reproduced Defects

The spring input validator imposed two absolute checks unrelated to the
caller-selected units:

- It rejected `stiffness + cubic_stiffness < 1e-12`, describing this as a
  positive tangent near unit extension. For the implemented force law
  `f = k*u + c*u^3`, the coefficients have different dimensions and the actual
  tangent is `k + 3*c*u^2`. That sum neither establishes stability nor preserves
  admissibility after a change of units.
- It rejected every element length at or below `1e-12`, even if the coordinate
  difference was finite and strictly nonzero. This scalar spring law uses
  caller-supplied force coefficients, not a shape-function division by length.

The initial five-test regression run produced four failures: force-unit
scaling, length-unit scaling, a stable small-displacement softening branch,
and a scaled branched network. The invalid-input control already passed.
The failures occurred during validation, before the numerical solver could
evaluate the valid equilibrium.

## Repair and Contract

The arbitrary coefficient-sum check was removed. Each linear stiffness must
still be finite and positive, and each cubic coefficient must be finite.
Element lengths must now be finite and strictly positive, without an absolute
length-unit floor. Zero length, coordinates collapsed by floating-point
rounding and overflowing coordinate differences remain invalid.

Existing topology restraints, finite assembly checks, explicit residual
tolerances, bounded Newton/backtracking work, linear-system conditioning
checks, cancellation and last-committed-state behavior are unchanged.
Protocol comments describe the coefficients' dimensions and warn that a
negative cubic coefficient does not guarantee stability over a load path.

No new engine dispatch logic, automatic unit parser/converter, runtime
dependency, regularization, tolerance relaxation or result-schema change was
introduced. This validation rule is specific to these scalar spring models;
it is not a replacement for geometric conditioning checks in continuum
elements.

## Independent Checks

For a positive length multiplier `L` and force multiplier `F`, the regressions
explicitly construct equivalent inputs:

```text
x' = L*x                  u' = L*u
load' = F*load            tolerance' = F*tolerance
k' = (F/L)*k              c' = (F/L^3)*c
gap' = L*gap              penalty' = (F/L)*penalty
```

These transformations are performed by test code, not implicitly by the
production solver. Output displacement, force, tangent, length and contact
force are compared with independently transformed expected values.

The five new solver tests cover:

- Linear and hardening laws, both load directions, with force multipliers
  from `2^-400` through `2^400`.
- Pure length and combined force/length changes, including lengths scaled
  by `2^-200`, using representable finite coefficients.
- The softening law `k=4`, `c=-16` at displacements `+/-0.125`. Its expected
  tangent is positive, `3.25`; the former coefficient-sum check rejected it.
- A five-node branched cyclic network with manufactured loads, with and
  without contact. Solver observation confirms that the non-contact network
  exercises the general dense fallback rather than only the path fast solver.
  Free-node forces are independently reassembled from returned element forces.
- Invalid zero/negative/nonfinite initial stiffness, nonfinite cubic
  coefficients, coincident coordinates and overflowing lengths, checked
  through all four borrowed/owned spring/contact entry points.

Workflow tests compare graph and direct execution across nine force/length
combinations for both operators. Rust headless tests cover hardening and the
stable softening branch through the real plan/route/engine path. Benchmark
controls check unchanged displacement under uniform force/stiffness scaling
and retain the earlier nonconvergence negative controls.

## Verification

Both platforms used Rust 1.88.0 and existing offline build caches:
local macOS AArch64 and the lab's isolated Linux x86_64 CI source directory.
Only changed Rust source/test files were synced. Installed services and
desktop packages were not modified.

| Selected group | macOS | Linux |
| --- | ---: | ---: |
| Spring/contact solver suites and mechanical convergence controls | 92 passed | 92 passed |
| Engine workflow suites | 9 passed | 9 passed |
| Rust headless plan-to-engine suite | 12 passed | 12 passed |
| Benchmark correctness controls | 4 passed | 4 passed |
| Total distinct selected tests per platform | 117 passed | 117 passed |

Local Clippy with `-D warnings` passed for the solver library and changed
solver, engine and CLI test targets, plus benchmark tests. Workspace formatting
and diff-whitespace checks passed. All source/config files touched in this
round remain below the 800-line limit.

Reproduce from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test nonlinear_spring_scaling --test nonlinear_spring_tolerance --test nonlinear_spring_globalization --test nonlinear_spring_topology --test nonlinear_spring_recovery --test nonlinear_spring_range_reliability --test nonlinear_spring_reliability --test nonlinear_spring_1d_closed_form --test nonlinear_spring_1d_review --test contact_gap_reliability --test contact_gap_1d_closed_form --test contact_gap_1d_refinement --test contact_gap_1d_review --test mechanical_convergence
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --test nonlinear_spring_recovery_workflow --test nonlinear_spring_topology_workflow --test nonlinear_spring_workflow --test contact_gap_workflow
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test nonlinear_spring_range_operator
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-benchmark spring_recovery
```

## Evidence and Limits

The existing `nonlinear-spring-range-local-reliability` profile and tensor
claim include the new tests, commands and report. Claim IDs, coordinate
assignments and grades are unchanged.

The validation-profile structural check passed for 59 profiles without
executing every profile command. Tensor structure and command checks passed
with 13 modules, 11 paradigms and no structural gaps. Overall readiness
remains blocked with 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

This is bounded local spring/contact numerical-range validation, not general nonlinear or industrial contact qualification.

The repair does not certify unstable softening, negative-tangent equilibria,
global uniqueness, snap-through/buckling or arbitrary conditioning. The old
unit-extension check did not provide those guarantees either. Coherent units
remain the caller's responsibility, and all inputs/outputs remain subject to
`f64` representability. These model-level tests do not establish continuum or
material-law validity at molecular scales.

Full-workspace, Windows, GUI, remote Agent transport and large-scale
performance suites were not rerun. Benchmark tests here validate numerical
consistency and failure classification, not throughput improvements.
