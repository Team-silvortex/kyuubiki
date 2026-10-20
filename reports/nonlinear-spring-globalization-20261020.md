# Bounded Newton Backtracking for Spring and Gap Contact

Date: 2026-10-20

## Reproduced Defect

The nonlinear spring and gap-contact load paths accepted every full Newton
correction. Strong hardening could turn a modest load into a very large first
displacement, followed by slow recovery, even when a known equilibrium existed.

Two manufactured-equilibrium regressions failed before the repair with one
load step, at most 12 Newton corrections and an absolute residual tolerance of
`1e-10`:

| Model | Known displacement | Residual after 12 full corrections |
| --- | ---: | ---: |
| Spring, linear stiffness 1, cubic stiffness `1e12` | `-1e-4` | about `1.546e6` |
| Same hardening with a gap stop | `1e-4` | about `5.216e6` |

The spring load is constructed independently as `u + c * u^3`. The contact
case adds a stop at `u/2`, penalty stiffness `1/u`, and contact force `0.5`.
Both cases now reach the prescribed equilibrium within the same correction
budget and tolerance. The tests also cover cubic stiffnesses `1e18` and `1e24`,
both spring load directions, and borrowed and owned public entry points.

## Algorithm and State Contract

`nonlinear_spring_1d.rs` now uses bounded residual backtracking:

1. Solve the existing tangent system for a Newton correction.
2. Evaluate the full step, then halve its scale at most 32 times.
3. Accept only if the original tolerance is met or the residual infinity norm
   decreases by the factor required by `1 - 1e-4 * scale`.
4. Reuse the accepted tangent and residual in the next Newton iteration.
5. If no candidate is accepted, return finite nonconvergence with the last
   committed load-step state and the last evaluated failed-trial residual.

There are at most 33 trial assemblies per Newton correction. The reported
iteration count remains the number of tangent solves/correction attempts, not
the number of merit evaluations. This is the same Newton correction budget,
not a claim of the same total assembly cost.

Three displacement buffers are reused for committed, current and candidate
states. Rejected trials are always recomputed from the current base state,
never accumulated. Reduced and previous tangent systems are released before
candidate assembly. A linear one-correction case is checked to perform exactly
two assemblies, initial and accepted, with no duplicate accepted-state work.

Cancellation and nonfinite arithmetic still propagate as errors. In
particular, an overflowing full step is not silently retried with a smaller
scale. Existing force, tangent, residual and output range checks remain in
force. The final allowed correction is still evaluated before reporting
iteration exhaustion.

Only the solver algorithm changed in production. Engine dispatch, Rust
headless routing, task/result protocols, load-step defaults, tolerances and
iteration limits were not changed. No artificial supports or stiffness
regularization were introduced.

## Regression Coverage

The new `nonlinear_spring_globalization` suite has 11 tests, including:

- 108 scalar contact cases cross-checked against an independent bracketed
  bisection implementation: six cubic stiffnesses, six signed loads and three
  gap/penalty combinations.
- A branched, cyclic six-node heterogeneous network with independently
  manufactured loads, with and without contact.
- Small stable softening branches whose tangent remains positive.
- Search-budget exhaustion before any committed load, and after a previously
  equilibrated contact load step.
- Cancellation during backtracking followed by clean in-process replay.
- Convergence on the last allowed correction and accepted-assembly reuse.

Three existing recovery fixtures needed explicit updates because the new
algorithm changes their attempted displacements:

- The one-correction cubic failure uses load magnitude 3 instead of 2; its
  evaluated damped residual is `1.875`. The old magnitude-2 model now succeeds
  and is retained as a new success regression.
- The unchanged partial-contact failure now reports `0.1875`, not the former
  full-step residual 5.
- The nonzero committed-residual rollback case uses gap `0.55` instead of
  `0.6`. The original gap-`0.6` model now succeeds and is retained separately.

None of these changes relaxes the tolerance or removes the failure-state
assertions. Engine, headless and benchmark fixtures check the revised
diagnostics. Nonconverged benchmark runs must remain failures, not fast
successes.

## Verification

Both platforms used Rust 1.88.0 and existing offline build caches:
local macOS AArch64 and Linux x86_64 in the lab's isolated CI source directory.
No installed runtime service was modified and no desktop package was rebuilt.

| Test group | macOS | Linux |
| --- | ---: | ---: |
| Spring/contact solver suites, including new backtracking and prior topology | 58 passed | 58 passed |
| Mechanical convergence cross-checks | 22 passed | 22 passed |
| Engine spring/contact workflow suites | 7 passed | 7 passed |
| Rust headless plan-to-engine suite | 10 passed | 10 passed |
| Benchmark failure/replay controls | 2 passed | 2 passed |
| Total distinct selected tests per platform | 99 passed | 99 passed |

The 108 scalar configurations are internal cases within one test, not 108
additional top-level tests. These counts describe the selected suites, not
whole-project test coverage.

Local Clippy with `-D warnings` passed for the solver library and changed
solver, engine and CLI test targets, plus benchmark test targets. Workspace
formatting and diff-whitespace checks passed. The validation-profile
structural check passed for 59 profiles; that check did not execute every
profile command.

Reproduce from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test nonlinear_spring_globalization --test nonlinear_spring_topology --test nonlinear_spring_recovery --test nonlinear_spring_range_reliability --test nonlinear_spring_reliability --test nonlinear_spring_1d_closed_form --test nonlinear_spring_1d_review --test contact_gap_reliability --test contact_gap_1d_closed_form --test contact_gap_1d_refinement --test contact_gap_1d_review --test mechanical_convergence
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --test nonlinear_spring_recovery_workflow --test nonlinear_spring_topology_workflow --test nonlinear_spring_workflow --test contact_gap_workflow
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test nonlinear_spring_range_operator
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-benchmark spring_recovery
```

## Evidence and Limits

The existing spring validation profile includes the new tests and report.
The two existing spring tensor claims are grouped in `runtime-spring.json`
instead of growing the near-limit `runtime-postprocess.json`. Claim IDs,
grades and coordinate assignments are preserved; no new readiness grade is
inferred from this change.

The tensor structure and command checks passed with 13 modules, 11 paradigms
and no structural gaps. Overall readiness remains blocked: 2 maturity gaps,
23 evidence-grade gaps and 18 P0 gaps remain. These metadata checks are not
additional numerical tests or release approval.

This is bounded local spring/contact committed-state validation, not durable restart, general nonlinear convergence or industrial contact qualification.

The bounded search deliberately rejects finite problems requiring more than
32 halvings. It does not introduce adaptive load stepping, arc-length
continuation, unstable softening/buckling tracking or initialization of a
floating body supported only by future contact activation. A finite residual
decrease does not certify global uniqueness, stability or conditioning.

Full-workspace, GUI, Windows, remote Agent transport, large-scale performance
and 1M-node suites were not rerun. The benchmark controls here validate failure
classification; they do not measure a throughput or latency improvement.
