# Composite Feedback Convergence: 2026-09-23

## Scope And Reproductions

Base revision: `65551b26`. This change tightens the Rust SDK feedback helpers
and the existing native composite research runner. It does not add a new
solver, dependency, GUI path, or material model.

Fourteen of the first fifteen targeted regressions failed before the fix:
ten SDK boundary/trace tests and all four initial native-loop tests. The
passing control retained the distinction between missing and unconverged data.

- The relative-change helper used an absolute difference when the previous
  value was below machine epsilon. A doubling from `1e-20` to `2e-20 W` was
  reported as a change of `1e-20`, not `1`.
- A real electrical/thermal loop stopped after two iterations at a thickness
  scale of `1e-20`. Joule loss fell from `1e-20` to `8e-21 W`, but the reported
  change was `2e-21` and the result incorrectly passed a `1e-9` relative limit.
- Negative changes could satisfy the public convergence predicate; infinite
  tolerances were not rejected there. Trace validation trusted reported
  residuals/changes instead of deriving them from the recorded values.
- Finite loss components could overflow their combined total, and summing four
  finite extreme temperatures could produce an infinite mean.
- `usize::MAX` iterations caused a capacity-overflow panic before the feedback
  specification was validated. A material failure omitted its iteration/stage.

## Corrected Contract

For finite non-negative power/conductivity values and positive previous value,
relative change is `abs(current - previous) / previous` at every scale.
Zero-to-zero is zero; zero-to-positive is a unit change. Negative/nonfinite
values or an unrepresentable ratio cannot converge. The predicate also requires
a valid feedback specification and finite, non-negative residuals/changes.

Trace assessment recomputes the maximum dielectric/regional temperature
residual, change of combined dielectric plus Joule power, and conductivity
changes matched by region ID. It rejects inconsistent metrics, duplicate or
missing region coverage, overflowed totals, and false convergence flags.
Metric agreement uses a relative comparison with no absolute epsilon floor.
Even within that comparison tolerance, the convergence decision uses recomputed
values so rounding allowance cannot move a failed threshold into a pass.
Pairwise overflow-safe midpoints preserve a finite four-node temperature mean.

The native loop no longer reserves memory from an unvalidated iteration count.
Feedback and projection failures identify the iteration and actual failed
stage. The caller's seed models remain unchanged, and a corrected request can
be retried. Exhausting a valid budget produces a `fail` convergence record,
not a successful fixed point.

## Analytic Cross-Check

The test uses a unit square with fixed `V=x`, zero-temperature left boundary,
unit reference conductivity/resistivity and `rho(theta)=1+theta`. Equal-quarter
Joule load lumping gives right-node temperature `1/(2*(1+theta))` and four-node
mean `theta=1/(4*(1+theta))`. Thus the fixed point is
`theta=(sqrt(2)-1)/2`, with power per unit thickness `1/(1+theta)`.
The temperature tolerance is deliberately loose (`1 C`) to isolate the loss
criterion; the relative loss tolerance remains `1e-9`.

All three thickness scales converge in 13 iterations.

| Thickness Scale | Mean Temperature | Power / Thickness | Iterations |
| --- | --- | --- | --- |
| `1e-20` | `0.207106781214` | `0.828427124855` | 13 |
| `1` | `0.207106781214` | `0.828427124855` | 13 |
| `1e20` | `0.207106781214` | `0.828427124855` | 13 |

Mean temperature and normalized power are checked against the independent
fixed point to absolute tolerance `1e-9`. The extreme thickness scales are
algebraic/numerical stress cases, not plausible material specimens. Another
real-solver case simultaneously enables dielectric loss, Joule loss,
temperature-dependent permittivity/loss tangent/thermal conductivity, and
under-relaxation, then checks its final nonlinear balance independently.

## Verification

- 14 public SDK regression tests pass in debug and optimized release builds.
- Six actual native feedback-loop tests pass in debug and optimized release builds.
- All 270 SDK library unit tests and 39 material-research entry tests pass.
- The registered conduction-screening profile executes all ten commands
  successfully, including the 16 preceding heat-projection regressions,
  conduction review/refinement/input/reference tests, RPC, engine workflow,
  SDK contract tests and these two new feedback targets. Overlapping runs are
  not summed into a unique-test coverage percentage.
- Strict Clippy passes for the SDK library/tests and CLI library with warnings
  denied and no lint exemptions. Formatting and `git diff --check` pass.
- Validation-profile structure, tensor structure, project organization and
  documentation inventory pass. Source/document limits remain 800/2,000 lines.
  Overall daji readiness remains blocked: four maturity, 16 evidence-grade
  and 11 P0 gaps are not promoted by this bounded claim.

Reproduction from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --test composite_feedback_reliability
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --release --test composite_feedback_reliability
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --lib composite_runtime_feedback::tests -- --nocapture
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --release --lib composite_runtime_feedback::tests -- --nocapture
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-material-explore
./scripts/kyuubiki check-operator-validation --execute --profile electric-conduction-plane-quad-screening --out tmp/composite-feedback-current-20260923.json
```

## Limits

This is bounded numerical screening, not general material qualification.
Trace consistency is not authenticity or independent proof of the underlying
solve. Total electrical loss remains an aggregate convergence metric; this
change does not add separate per-mechanism or per-conductor convergence gates.
The material laws remain linear temperature scalings within an existing
fixed-point iteration. Runaway, nonlinear constitutive validity, subcell source
quadrature, automatic interface heat transfer and general mixed-field stability
are not established by this test. There is no remote, installed-app, Windows,
million-node or new performance benchmark claim.

Disposable development results are not backed up or migrated. A previously
accepted low-power feedback result must be recomputed if it will be reused.
