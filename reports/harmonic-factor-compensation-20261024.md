# Harmonic Elimination Factor Compensation

Date: 2026-10-24

Source base: `c4cc2f96`, retaining the uncommitted
[Schur-update follow-up](harmonic-schur-update-20261024.md). Rust workspace package
metadata remains 3.7.0. Eligible elimination steps now carry one residual
correction alongside the rounded factor and use it consistently in coefficient
and RHS updates. Production changes remain private Solver arithmetic; Engine,
Agent, TaskIR, operator contracts and deployment configuration are unchanged.
No version bump, publication or desktop rebuild is included.

## Failures Before Repair

Two private tests and one public operator test failed before implementation.
In a manufactured system `[[3,-1],[-1,q+w]]`, where `q=round(1/3)` and
`w=2^-20`, the exact Schur coefficient is `w-2^-54/3`. A correctly rounded
but stored factor still loses the division tail: subsequent accurate product
updates evaluate the altered coefficient `w`. The path solver returns a tip
unknown of one rather than `3*w/(3*w-2^-54)`, approximately
`1.0000000000194025`.

A complex leading row phase `1+i` makes the weighted leading pivot preferred
in both dense and path implementations, so both exercise non-dyadic factors.
The unmodified solve-level regression failed before repair. Its later controls
include row permutations and common binary scales, not just a replacement
factor helper evaluated in isolation.

The public fixture is a two-spring oscillator at angular frequency one.
The spring stiffnesses are three and one, the middle mass is one, the tip load
is `w`, and tip mass is `m0-w`, where `m0=round(2/3)`. Its represented dynamic
matrix is `[[3,-1],[-1,1-m0+w]]`. The independent reference honors the actual
binary64 mass, not the ideal rational `2/3`:
`tip=3*w/(3*w+2^-53)`.

At `w=2^-20`, the old successful amplitude is `0.9999999999417923`; the
reference is approximately `0.9999999999611949`, a difference of about
`1.94e-11`. This is a sensitive near-resonant model, and the old result still
satisfies the original backward-residual tolerance. It is not evidence that
physical input uncertainty or conditioning has disappeared. The final public
test checks three `w` values against rationally derived references within four
binary64 epsilons of relative error.

## Shared Two Term Factor

For represented numerator `l` and pivot `p`, construction first retains the
existing rounded quotient `q`. It computes a rounded residual `r=l-q*p`, using
scalar FMA for axial pivots or the existing complete-expression difference
machinery for general complex pivots. A second rounded quotient `r/p` becomes
the correction. Neither the residual nor the correction is an exact rational
factor certificate.

The factor is a transient pair of complex values, at most 32 bytes, not a
stored vector of factors or an original-matrix snapshot. An exact-factor
positive control has zero correction and retains the previous update path.
Eligible updates evaluate both `q*other` and `correction*other` with the shared
fused/replay policy. Coefficient updates retain exact-restoration guards;
RHS updates retain the existing scaled representation when replay is needed.

Compensation construction requires each nonzero component of `l`, `p` and `q`
to lie in the inclusive headroom band `[2^-100,2^100]`. This conservative,
read-only policy leaves exponent room for residual and correction products.
The row operands must also satisfy that band after existing preparation;
common raw input scales outside it can still normalize into the supported band.

Eligibility is decided for the entire step before row mutation. A shared
preflight checks every coefficient pair that will be updated and both RHS
operands. Any unsupported component, nonfinite restoration or wide RHS disables
correction for the whole step. Coefficients must not use a compensated factor
while the RHS silently uses only its rounded value. Tests cover an unsupported
later coefficient, a finite out-of-band RHS and a wide RHS, followed by uniform
legacy matrix/RHS readback.

Outside this scope, the whole step retains the preceding range-checked rounded
factor path, including its known rounding limitations. Numerical errors and
cancellation encountered while constructing an eligible correction propagate;
they are not caught and converted into a successful uncompensated result.
Long preflight scans check cancellation every 1,024 pairs.

Both dense elimination and ordinary/swapped path branches share the same
step-local factor for diagonals, generated fill-in and RHS. Pivot constants,
row-weight units and original-equation acceptance are unchanged. No pivot
regularization, physical tolerance relaxation or protocol-visible state is added.

Residual evaluation, a second division, row preflight and additional updates
cost work. There is no new persistent factor storage or dependency, but this is
not evidence of performance neutrality or speedup. Whole-model scale performance
requires separate measurement.

## Regression Coverage

Twelve new test functions cover seven private units, two public Solver tests
and three Engine integration tests:

- A real path counterexample and complex leading-row controls in both
  factorizations, with row swaps and three common binary scales.
- An adjacent-swap case with generated second-superdiagonal fill, an independently
  known third unknown and consistent compensated coefficient/RHS updates.
- One shared correction applied to both matrix and RHS values for signed real,
  imaginary and general complex pivots, plus exact-factor zero-correction control.
- Inclusive headroom edges, unsupported whole-step decisions, extended RHS
  behavior, exact quantum preservation, partial-quantum rejection, invalid
  operands and true range failure.
- Cancellation during residual construction, a 2,050-pair step preflight and
  either two-term replay component, with fresh same-input reuse.
- Public references at `w=2^-18`, `2^-20` and `2^-24`, both load signs, reversed
  element orientation, borrowed/owned equality and finite force readback.
- Input/result byte roundtrips and Engine dispatch/workflow equality. Factor
  and result cancellation or later frequency range failure publishes no
  successful prefix; explicit repaired-input reuse succeeds.

Four-epsilon solution checks are bounded reference comparisons, not exact
whole-solve bits or an all-input forward-accuracy claim. These are small
analytical models, not independent material qualification. Cooperative
cancellation is not process-kill or host-power-loss recovery evidence.

## Platform Verification

Pinned Rust 1.88.0 and locked offline dependencies reuse existing build
directories on macOS AArch64 and Linux x86_64. Linux uses the preceding source
overlay, not a clean Git checkout or installed runtime. Both existing source
files matched their preceding digests before transfer, and all five new files
were absent. All seven transferred files matched final local SHA-256 digests.
No service deployment or credentials changed.

| Completed final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 878 | 878 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Existing Engine constraint integration | 16 | 16 | Selected harmonic scope |
| Previous quotient, division, row and Schur integrations | 25 | 25 | 15 Engine and 10 public Solver tests |
| New compensated-factor integrations | 5 | 5 | 3 Engine and 2 public Solver tests |
| Existing public complex-force integration | 7 | 7 | Selected force scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets |
| Total | 1,672 | 1,672 | Not full workspace, installed or all-physics qualification |

macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. Historical reports retain their earlier source,
algorithms and counts.

The native validation catalog passes with 100 registered profiles; catalog
inspection does not execute all of them. Executing the new profile passes its
19 selected tests. Documentation inventory and all 26 book HTML files pass.
The project organization audit passes the 800-line source and 2,000-line
document limits with zero tracked debt. The coverage tensor passes structural
and command checks across 13 modules and 11 paradigms, with zero structural
gaps. Its two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps remain;
overall daji qualification is still blocked.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_factor_compensation
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_schur_update
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_factor_compensation
```

Profile: `harmonic-factor-compensation-local-reliability`, in the registered
`harmonic-cancellation.json` shard. Its three commands select seven new factor
units, seven existing Schur units and five new integration tests. These 19 tests
are a subset of the table, not additional tests to sum.

## Remaining Boundary

This is bounded harmonic elimination-factor residual compensation, not exact factorization, arbitrary-range compensation or unrounded-expression division, a whole-solver forward-error certificate, independent physical qualification or durable restart.

The correction, successive updated coefficients and row numerators still round.
Unsupported and wide-RHS steps inherit the preceding path's precision limits.
Condition-aware forward-error budgets, exact full-expression division, independent
physical/material references, large-scale performance, installed/SDK journeys
and persisted recovery require separate evidence. No qualification threshold or
release grade is promoted by these local controls.
