# Harmonic Replayed Row Quotient Rounding

Date: 2026-10-24

Source base: `c4cc2f96`, retaining the uncommitted
[Schur update](harmonic-schur-update-20261024.md) and
[factor compensation](harmonic-factor-compensation-20261024.md) follow-ups.
Rust package metadata remains 3.7.0. Replayed harmonic substitution rows now
retain their admitted exact integer numerator until final quotient rounding.
Ordinary fused rows keep the previous fast route. The implementation remains
private Solver arithmetic, with no Engine, Agent, TaskIR, SDK or operator
contract change, version bump, deployment or desktop rebuild.

## Reproduced Double Rounding

Two test functions failed before implementation: one at the row helper and one
through dense and path solve entry points. Let `b=1+2^-30`, and let represented
binary64 operands `a` and `x` have bit patterns `0x3ff0000000021abf` and
`0x3feffffffff91180`. Their exact row numerator is
`N=b-a*x=301425993429140949475/2^98`.

The previous replay accumulates `N` exactly but rounds it to a scaled binary64
before division. For a pivot of 51, that produces bits `0x3db48169b9b9de57`;
rounding `N/51` once gives the adjacent value `0x3db48169b9b9de56`. The complex
pivot `51+4i` also exposes adjacent-value errors: direct rational references for
`51*N/2617` and `-4*N/2617` give bits `0x3db461519a11f21b` and
`0xbd79933939d03ee1`. These constants were computed independently with integer
rational arithmetic, not by reusing Solver quotient code.

The difference is one output ULP in this manufactured counterexample, not an
observed large physical-model error. Fixing it prevents an avoidable second
rounding at this local boundary; it does not erase conditioning or upstream
matrix and solution errors.

## Exact Integer Handoff

The existing two-pass product accumulation still performs its reversible
alignment checks and uses the same 18-limb signed integer storage. Its new
consuming handoff returns the signed magnitude without first rounding it.
Legacy coefficient and RHS updates still call the existing rounded finish.

Only substitution rows already selected for replay pass the two exact component
sums into the exceptional complex quotient machinery. That machinery reuses its
checked 70-limb magnitudes, exact squared denominator and ties-to-even integer
quotient rounding. Checked scalar multiplication and signed alignment form
`R*p.re+I*p.im` and `I*p.re-R*p.im` directly from the retained sums. This also
serves purely real and imaginary pivots, including final subnormal rounding.

Reversible alignment, fixed-capacity carries and shifts, invalid operands,
nonzero solutions that round to zero and true overflow remain guarded. The
physical residual gate and weighted pivot floor are unchanged. Cancellation
propagates through accumulation, the new integer multiplication boundaries and
both output quotient phases; no caught error becomes an ordinary-path success.

Storage is fixed and local, with no heap-allocated precision engine, matrix
snapshot or dependency. Ordinary positive controls observe no integer replay.
Exceptional axial replay does more integer work than its preceding scaled
scalar division, so this change makes no speedup or performance-neutral claim.

## Regression Coverage

Fourteen new test functions cover nine private tests, two public Solver tests
and three Engine integrations:

- Rational counterexamples with real and complex pivots, four numerator phases,
  three binary scales, padded terms and different cancellation orderings.
- Dense and path entry points, real, imaginary and general complex pivots,
  and an adjacent row swap with bit-exact component references.
- Minimum-subnormal midpoint sides and ties, including a nonzero result just
  above the zero threshold and rejection at or below it.
- Wide RHS cancellation before division, preserved normalization rejection,
  true overflow, invalid later operands and fresh input reuse.
- Integer carry, capacity, signed alignment, exact-sum handoff and poisoned
  accumulation controls, plus an unchanged ordinary-path positive control.
- Cancellation at either accumulation component, the denominator, every new
  scalar multiplication boundary and final quotient output phases.
- Public balanced-load springs with three weak loads, three common stiffness
  and load scales, both signs and reversed edges. The analytical displacements
  are `w` and `1+w`; spring force amplitudes are `scale*w` and `scale`.
- Borrowed/owned and serialized Engine dispatch/workflow equality, explicit
  reachability of the new integer phases, cancellation without a successful
  workflow prefix, and late-frequency rejection followed by repaired reuse.

The public spring fixture is an exact analytical positive control that enters
the new replay route, not a claim that its previous output was wrong. The two
before-repair failures are the private rational helper and solve-level tests.
Public tests retain complete-node and force readback rather than only checking
that the operator returned success.

## Verification

Pinned Rust 1.88.0 with locked offline dependencies reuses the existing build
directory. On macOS AArch64, the final Rust source passes 887 Solver library
tests, 711 Engine library tests and 88 selected integration tests: 1,686 passed.
Nine Solver tests and one Engine test remain ignored, and 292
`modal_frame_spectrum::` tests are excluded. These counts are selected scope,
not full-workspace or all-physics qualification. All-target Solver/Engine
Clippy with warnings denied also passes.

Linux x86_64 repeats the same 887 library, 711 library and 88 integration
passes, totaling 1,686, with the same ignored and excluded scopes. The existing
source overlay is not a clean checkout or installed runtime. Before transfer,
the three changed files matched the preceding factor-overlay or committed
baseline digests, and the five new files were absent. All eight transferred
files matched the final macOS SHA-256 digests. Tests reuse the existing remote
build directory with two build jobs; no service deployment or configuration
changed.

Rust formatting and diff whitespace checks pass. Native catalog inspection
passes with 101 registered profiles, without executing all profiles. Executing
the new profile passes its 21 selected tests. Documentation inventory, all 26
book HTML files and the project organization audit pass; source/config limits
remain 800 lines and document limits 2,000, with zero tracked debt. The tensor
passes structural and command checks for 13 modules and 11 paradigms with zero
structural gaps. Its two maturity, 23 evidence-grade and 18 P0 gaps remain;
overall daji qualification is still blocked.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_row_quotient
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_general_division
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_row_quotient
```

The registered `harmonic-row-quotient-local-reliability` profile selects nine
new private tests, seven existing exact-division controls and five new public
and Engine tests. Its 21 tests are a subset of the larger regression, not an
additional count. A separate harmonic-row evidence shard keeps the existing
constraint shard below the source/config line limit.

## Remaining Boundary

This is bounded exact quotient rounding of admitted replayed harmonic row expressions, not correct rounding of ordinary rows, exact assembly or factorization, a whole-solver forward-error certificate, independent physical qualification or durable restart.

The exact expression starts from represented row coefficients, solved unknowns
and the current RHS. Earlier RHS updates, elimination and assembly still round;
the ordinary fused path still uses its local routing budget. Alignment failures
still reject unsupported spans rather than promising arbitrary precision.
Large-model performance, installed SDK journeys, independent material references
and process-kill or power-loss recovery require their own evidence. This local
follow-up does not promote release qualification or change its thresholds.
